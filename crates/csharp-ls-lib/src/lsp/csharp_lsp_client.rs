use crate::{
    CSharpLsLibError, CSharpLsLibResult,
    lsp::{LaunchConfig, file_uri},
};

use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{env, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
    time,
};

pub(crate) struct CSharpLspClient {
    config: LaunchConfig,
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_request_id: i64,
}

impl CSharpLspClient {
    pub(crate) fn spawn(config: LaunchConfig) -> CSharpLsLibResult<Self> {
        let mut command = Command::new(&config.binary);
        command
            .arg("--loglevel")
            .arg(&config.log_level)
            .arg("--solution")
            .arg(&config.solution);
        if !config.features.is_empty() {
            command.arg("--features").arg(config.features.join(","));
        }
        if let Ok(rpc_log_path) = env::var("SEMANTIC_GRAPH_CSHARP_LS_RPC_LOG")
            && !rpc_log_path.trim().is_empty()
        {
            command.arg("--rpclog").arg(rpc_log_path);
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);

        let mut child = command
            .spawn()
            .map_err(|source| spawn_error(&config.binary, source))?;
        let stdin = child.stdin.take().ok_or_else(|| {
            CSharpLsLibError::setup(
                &config.binary,
                "language server stdin was not captured after spawn",
            )
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            CSharpLsLibError::setup(
                &config.binary,
                "language server stdout was not captured after spawn",
            )
        })?;

        Ok(Self {
            config,
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_request_id: 1,
        })
    }

    pub(crate) async fn initialize(&mut self) -> CSharpLsLibResult<()> {
        let root_uri = file_uri(
            self.config
                .solution
                .parent()
                .unwrap_or(self.config.solution.as_path()),
        )?;
        let params = json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": {
                "workspace": {
                    "configuration": true
                },
                "textDocument": {
                    "documentSymbol": {
                        "hierarchicalDocumentSymbolSupport": true
                    },
                    "callHierarchy": {
                        "dynamicRegistration": false
                    },
                    "references": {
                        "dynamicRegistration": false
                    }
                },
                "window": {
                    "workDoneProgress": true
                }
            },
            "trace": "off"
        });
        let _result: Value = self
            .request_value("initialize", params, self.config.startup_timeout_ms)
            .await?;
        self.notify("initialized", Some(json!({}))).await
    }

    pub(crate) async fn request<P, T>(&mut self, method: &str, params: P) -> CSharpLsLibResult<T>
    where
        P: Serialize,
        T: DeserializeOwned,
    {
        let params = serde_json::to_value(params)
            .map_err(|source| CSharpLsLibError::json("serialize LSP request params", source))?;
        let value = self
            .request_value(method, params, self.config.request_timeout_ms)
            .await?;
        serde_json::from_value(value)
            .map_err(|source| CSharpLsLibError::json("deserialize LSP response", source))
    }

    pub(crate) async fn request_value(
        &mut self,
        method: &str,
        params: Value,
        timeout_ms: u64,
    ) -> CSharpLsLibResult<Value> {
        let request_id = self.allocate_request_id(method)?;
        let message = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": params,
        });
        let timeout = Duration::from_millis(timeout_ms);
        let method_name = method.to_string();

        match time::timeout(timeout, async {
            self.write_json(&message).await?;
            self.read_response(&method_name, request_id).await
        })
        .await
        {
            Ok(result) => result,
            Err(_) => Err(CSharpLsLibError::timeout(
                method,
                Some(request_id),
                timeout_ms,
            )),
        }
    }

    pub(crate) async fn notify(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> CSharpLsLibResult<()> {
        let mut message = json!({
            "jsonrpc": "2.0",
            "method": method,
        });
        if let Some(params) = params {
            message["params"] = params;
        }
        self.write_json(&message).await
    }

    pub(crate) async fn shutdown(mut self) -> CSharpLsLibResult<()> {
        if let Err(error) = self
            .request_value("shutdown", Value::Null, self.config.request_timeout_ms)
            .await
        {
            let _kill_result = self.kill().await;
            return Err(error);
        }

        if let Err(error) = self.notify("exit", None).await {
            let _kill_result = self.kill().await;
            return Err(error);
        }

        self.wait_for_exit().await
    }

    async fn read_response(
        &mut self,
        method: &str,
        expected_request_id: i64,
    ) -> CSharpLsLibResult<Value> {
        loop {
            let value = read_framed_json(&mut self.stdout).await?;

            if is_server_request(&value) {
                self.handle_server_request(&value).await?;
                continue;
            }

            if value.get("id").is_none() {
                continue;
            }

            let response_id = value.get("id").and_then(Value::as_i64).ok_or_else(|| {
                CSharpLsLibError::protocol(
                    method,
                    Some(expected_request_id),
                    "response id was not an integer",
                )
            })?;

            if response_id != expected_request_id {
                return Err(CSharpLsLibError::protocol(
                    method,
                    Some(expected_request_id),
                    format!("received response for unexpected request id {response_id}"),
                ));
            }

            if let Some(error) = value.get("error") {
                return Err(CSharpLsLibError::protocol(
                    method,
                    Some(expected_request_id),
                    format!("server returned error {error}"),
                ));
            }

            return Ok(value.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    async fn handle_server_request(&mut self, value: &Value) -> CSharpLsLibResult<()> {
        let id = value.get("id").cloned().ok_or_else(|| {
            CSharpLsLibError::protocol("server-request", None, "server request was missing id")
        })?;
        let method = value.get("method").and_then(Value::as_str).ok_or_else(|| {
            CSharpLsLibError::protocol("server-request", None, "server request was missing method")
        })?;
        let result = match method {
            "workspace/configuration" => workspace_configuration_response(&self.config),
            "client/registerCapability" | "window/workDoneProgress/create" => Value::Null,
            _ => Value::Null,
        };
        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        });
        self.write_json(&response).await
    }

    async fn write_json(&mut self, value: &Value) -> CSharpLsLibResult<()> {
        write_json_message(&mut self.stdin, value).await
    }

    fn allocate_request_id(&mut self, method: &str) -> CSharpLsLibResult<i64> {
        let request_id = self.next_request_id;
        self.next_request_id = self.next_request_id.checked_add(1).ok_or_else(|| {
            CSharpLsLibError::protocol(method, Some(request_id), "request id overflow")
        })?;

        Ok(request_id)
    }

    async fn wait_for_exit(&mut self) -> CSharpLsLibResult<()> {
        let timeout_ms = self.config.request_timeout_ms;
        match time::timeout(Duration::from_millis(timeout_ms), self.child.wait()).await {
            Ok(Ok(status)) if status.success() => Ok(()),
            Ok(Ok(status)) => Err(CSharpLsLibError::setup(
                &self.config.binary,
                format!("language server exited with status {status}"),
            )),
            Ok(Err(source)) => Err(CSharpLsLibError::io(
                "wait for csharp-ls process",
                None,
                source,
            )),
            Err(_) => {
                let kill_result = self.kill().await;
                kill_result?;
                Err(CSharpLsLibError::timeout("process-exit", None, timeout_ms))
            }
        }
    }

    async fn kill(&mut self) -> CSharpLsLibResult<()> {
        self.child
            .kill()
            .await
            .map_err(|source| CSharpLsLibError::io("kill csharp-ls process", None, source))
    }
}

pub(crate) async fn read_framed_json<R>(reader: &mut R) -> CSharpLsLibResult<Value>
where
    R: AsyncBufRead + Unpin,
{
    let content_length = read_headers(reader).await?;
    let mut body = vec![0_u8; content_length];
    reader
        .read_exact(&mut body)
        .await
        .map_err(|source| CSharpLsLibError::io("read LSP message body", None, source))?;

    serde_json::from_slice(&body)
        .map_err(|source| CSharpLsLibError::json("parse LSP message", source))
}

async fn read_headers<R>(reader: &mut R) -> CSharpLsLibResult<usize>
where
    R: AsyncBufRead + Unpin,
{
    let mut content_length = None;
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .await
            .map_err(|source| CSharpLsLibError::io("read LSP header", None, source))?;
        if bytes == 0 {
            return Err(CSharpLsLibError::protocol(
                "read-message",
                None,
                "language server closed stdout",
            ));
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            return content_length.ok_or_else(|| {
                CSharpLsLibError::protocol("read-message", None, "missing Content-Length header")
            });
        }
        let Some((name, value)) = trimmed.split_once(':') else {
            return Err(CSharpLsLibError::protocol(
                "read-message",
                None,
                format!("malformed LSP header {trimmed}"),
            ));
        };
        if name.eq_ignore_ascii_case("Content-Length") {
            content_length = Some(value.trim().parse::<usize>().map_err(|source| {
                CSharpLsLibError::protocol(
                    "read-message",
                    None,
                    format!("invalid Content-Length header: {source}"),
                )
            })?);
        }
    }
}

async fn write_json_message<W>(writer: &mut W, value: &Value) -> CSharpLsLibResult<()>
where
    W: AsyncWrite + Unpin,
{
    let body = serde_json::to_vec(value)
        .map_err(|source| CSharpLsLibError::json("serialize LSP message", source))?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer
        .write_all(header.as_bytes())
        .await
        .map_err(|source| CSharpLsLibError::io("write LSP header", None, source))?;
    writer
        .write_all(&body)
        .await
        .map_err(|source| CSharpLsLibError::io("write LSP message body", None, source))?;
    writer
        .flush()
        .await
        .map_err(|source| CSharpLsLibError::io("flush LSP message", None, source))
}

fn workspace_configuration_response(config: &LaunchConfig) -> Value {
    let features = config
        .features
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    json!([{
        "logLevel": config.log_level,
        "useMetadataUris": features.contains(&"metadata-uris"),
        "razorSupport": features.contains(&"razor-support"),
        "solutionPathOverride": config.solution,
    }])
}

fn is_server_request(value: &Value) -> bool {
    value.get("id").is_some() && value.get("method").is_some()
}

fn spawn_error(binary: &std::path::Path, source: std::io::Error) -> CSharpLsLibError {
    if source.kind() == std::io::ErrorKind::NotFound {
        CSharpLsLibError::missing_binary(binary)
    } else {
        CSharpLsLibError::io(
            "spawn csharp-ls process",
            Some(binary.to_path_buf()),
            source,
        )
    }
}
