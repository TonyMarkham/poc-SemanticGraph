use std::process::Stdio;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{
    AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader,
};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::time;

use crate::error::{ExtractError, Result};
use crate::model::ProviderId;

pub struct LspStdioClient {
    provider: ProviderId,
    process_name: String,
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_request_id: i64,
}

impl LspStdioClient {
    pub fn spawn(process_name: &str, provider: ProviderId) -> Result<Self> {
        let mut command = Command::new(process_name);
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);

        let mut child = command.spawn().map_err(|source| {
            ExtractError::io(
                format!("spawn language server {process_name}"),
                None,
                source,
            )
        })?;
        let stdin = child.stdin.take().ok_or_else(|| {
            ExtractError::process(
                provider.as_str(),
                process_name,
                "language server stdin was not captured",
            )
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            ExtractError::process(
                provider.as_str(),
                process_name,
                "language server stdout was not captured",
            )
        })?;

        Ok(Self {
            provider,
            process_name: process_name.to_string(),
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_request_id: 1,
        })
    }

    pub async fn request(&mut self, method: &str, params: Value, timeout_ms: u64) -> Result<Value> {
        let request_id = self.allocate_request_id(method)?;
        let message = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": params,
        });
        let timeout = Duration::from_millis(timeout_ms);
        let provider = self.provider.as_str().to_string();
        let method_name = method.to_string();

        match time::timeout(timeout, async {
            self.write_json(&message).await?;
            self.read_response(&method_name, request_id).await
        })
        .await
        {
            Ok(result) => result,
            Err(_) => Err(ExtractError::timeout(
                provider,
                method,
                Some(request_id),
                timeout_ms,
            )),
        }
    }

    pub async fn notify(
        &mut self,
        method: &str,
        params: Option<Value>,
        timeout_ms: u64,
    ) -> Result<()> {
        let mut message = json!({
            "jsonrpc": "2.0",
            "method": method,
        });
        if let Some(params) = params {
            message["params"] = params;
        }
        let timeout = Duration::from_millis(timeout_ms);

        match time::timeout(timeout, self.write_json(&message)).await {
            Ok(result) => result,
            Err(_) => Err(ExtractError::timeout(
                self.provider.as_str(),
                method,
                None,
                timeout_ms,
            )),
        }
    }

    pub async fn shutdown(mut self, timeout_ms: u64) -> Result<()> {
        if let Err(error) = self.request("shutdown", Value::Null, timeout_ms).await {
            let _kill_result = self.kill().await;
            return Err(error);
        }

        if let Err(error) = self.notify("exit", None, timeout_ms).await {
            let _kill_result = self.kill().await;
            return Err(error);
        }

        self.wait_for_exit(timeout_ms).await
    }

    async fn read_response(&mut self, method: &str, expected_request_id: i64) -> Result<Value> {
        loop {
            let value = read_framed_json(&mut self.stdout).await?;

            if value.get("id").is_none() {
                continue;
            }

            let response_id = value.get("id").and_then(Value::as_i64).ok_or_else(|| {
                ExtractError::protocol(
                    self.provider.as_str(),
                    method,
                    Some(expected_request_id),
                    "response id was not an integer",
                )
            })?;

            if response_id != expected_request_id {
                return Err(ExtractError::protocol(
                    self.provider.as_str(),
                    method,
                    Some(expected_request_id),
                    format!("received response for unexpected request id {response_id}"),
                ));
            }

            if let Some(error) = value.get("error") {
                return Err(ExtractError::protocol(
                    self.provider.as_str(),
                    method,
                    Some(expected_request_id),
                    format!("server returned error {error}"),
                ));
            }

            if let Some(result) = value.get("result") {
                return Ok(result.clone());
            }

            return Err(ExtractError::protocol(
                self.provider.as_str(),
                method,
                Some(expected_request_id),
                "response had neither result nor error",
            ));
        }
    }

    async fn write_json(&mut self, value: &Value) -> Result<()> {
        write_json_message(&mut self.stdin, value).await
    }

    fn allocate_request_id(&mut self, method: &str) -> Result<i64> {
        let request_id = self.next_request_id;
        self.next_request_id = self.next_request_id.checked_add(1).ok_or_else(|| {
            ExtractError::protocol(
                self.provider.as_str(),
                method,
                Some(request_id),
                "request id overflow",
            )
        })?;

        Ok(request_id)
    }

    async fn wait_for_exit(&mut self, timeout_ms: u64) -> Result<()> {
        match time::timeout(Duration::from_millis(timeout_ms), self.child.wait()).await {
            Ok(Ok(status)) if status.success() => Ok(()),
            Ok(Ok(status)) => Err(ExtractError::process(
                self.provider.as_str(),
                &self.process_name,
                format!("language server exited with status {status}"),
            )),
            Ok(Err(source)) => Err(ExtractError::io(
                "wait for language server process",
                None,
                source,
            )),
            Err(_) => {
                let kill_result = self.kill().await;
                kill_result?;
                Err(ExtractError::timeout(
                    self.provider.as_str(),
                    "process-exit",
                    None,
                    timeout_ms,
                ))
            }
        }
    }

    async fn kill(&mut self) -> Result<()> {
        self.child.kill().await.map_err(|source| {
            ExtractError::io(
                format!("kill language server {}", self.process_name),
                None,
                source,
            )
        })
    }
}

pub async fn read_framed_json<R>(reader: &mut R) -> Result<Value>
where
    R: AsyncBufRead + Unpin,
{
    let content_length = read_headers(reader).await?;
    let mut body = vec![0_u8; content_length];
    reader
        .read_exact(&mut body)
        .await
        .map_err(|source| ExtractError::io("read LSP message body", None, source))?;

    serde_json::from_slice(&body).map_err(|source| ExtractError::json("parse LSP message", source))
}

async fn read_headers<R>(reader: &mut R) -> Result<usize>
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
            .map_err(|source| ExtractError::io("read LSP header", None, source))?;

        if bytes == 0 {
            return Err(ExtractError::protocol(
                "lsp-stdio",
                "read-message",
                None,
                "unexpected EOF while reading headers",
            ));
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }

        let (name, value) = trimmed.split_once(':').ok_or_else(|| {
            ExtractError::protocol(
                "lsp-stdio",
                "read-message",
                None,
                format!("malformed header line {trimmed:?}"),
            )
        })?;

        if name.eq_ignore_ascii_case("Content-Length") {
            let parsed = value.trim().parse::<usize>().map_err(|source| {
                ExtractError::protocol(
                    "lsp-stdio",
                    "read-message",
                    None,
                    format!("invalid Content-Length: {source}"),
                )
            })?;
            content_length = Some(parsed);
        }
    }

    content_length.ok_or_else(|| {
        ExtractError::protocol(
            "lsp-stdio",
            "read-message",
            None,
            "missing Content-Length header",
        )
    })
}

async fn write_json_message<W>(writer: &mut W, value: &Value) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let payload = serde_json::to_vec(value)
        .map_err(|source| ExtractError::json("serialize LSP message", source))?;
    let header = format!("Content-Length: {}\r\n\r\n", payload.len());

    writer
        .write_all(header.as_bytes())
        .await
        .map_err(|source| ExtractError::io("write LSP header", None, source))?;
    writer
        .write_all(&payload)
        .await
        .map_err(|source| ExtractError::io("write LSP body", None, source))?;
    writer
        .flush()
        .await
        .map_err(|source| ExtractError::io("flush LSP message", None, source))
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use tokio::io::{AsyncWriteExt, BufReader, duplex};

    use super::*;

    #[tokio::test]
    async fn framing_parser_reads_one_valid_response() -> std::result::Result<(), Box<dyn Error>> {
        let value =
            read_one(b"Content-Length: 38\r\n\r\n{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":null}")
                .await?;

        assert_eq!(value["id"], 1);
        assert!(value.get("result").is_some());
        Ok(())
    }

    #[tokio::test]
    async fn framing_parser_allows_notification_before_response()
    -> std::result::Result<(), Box<dyn Error>> {
        let input = concat!(
            "Content-Length: 46\r\n\r\n",
            "{\"jsonrpc\":\"2.0\",\"method\":\"window/logMessage\"}",
            "Content-Length: 36\r\n\r\n",
            "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":[]}"
        );
        let (mut writer, reader) = duplex(512);
        writer.write_all(input.as_bytes()).await?;
        drop(writer);

        let mut reader = BufReader::new(reader);
        let notification = read_framed_json(&mut reader).await?;
        let response = read_framed_json(&mut reader).await?;

        assert_eq!(notification["method"], "window/logMessage");
        assert_eq!(response["id"], 2);
        Ok(())
    }

    #[tokio::test]
    async fn framing_parser_rejects_malformed_headers() -> std::result::Result<(), Box<dyn Error>> {
        let result = read_one(b"Content-Length nope\r\n\r\n{}").await;

        assert!(matches!(result, Err(ExtractError::JsonRpcProtocol { .. })));
        Ok(())
    }

    #[tokio::test]
    async fn framing_parser_rejects_malformed_json() -> std::result::Result<(), Box<dyn Error>> {
        let result = read_one(b"Content-Length: 1\r\n\r\n{").await;

        assert!(matches!(result, Err(ExtractError::Json { .. })));
        Ok(())
    }

    async fn read_one(bytes: &[u8]) -> Result<Value> {
        let (mut writer, reader) = duplex(512);
        writer
            .write_all(bytes)
            .await
            .map_err(|source| ExtractError::io("write test fixture", None, source))?;
        drop(writer);

        let mut reader = BufReader::new(reader);
        read_framed_json(&mut reader).await
    }
}
