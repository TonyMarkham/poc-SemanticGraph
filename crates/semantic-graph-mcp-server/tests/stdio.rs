use semantic_graph_db_manager::{
    EdgeEvidenceInput, EdgeInput, FileInput, FtsWriteBatchDocumentInput, FtsWriteBatchInput,
    NodeInput, OccurrenceInput, RouteObservationInput, RouteStatusCompleteInput,
    RouteStatusStartInput, TextRange, WriteHandle, WriteManager, edge_id, node_id,
};
use semantic_graph_search_tantivy::{TantivyFtsDocument, TantivyFtsIndex, TantivyFtsIndexUpdate};

use serde_json::{Value, json};
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStdin, ChildStdout, Command},
    time::{Duration, timeout},
};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[tokio::test]
async fn stdio_server_handles_phase_two_tools_and_resources_from_config()
-> Result<(), Box<dyn Error>> {
    let fixture = Fixture::seed("config").await?;
    write_config(&fixture.root, &fixture.database_path)?;
    let mut server = StdioServer::start(&fixture.root, Vec::new()).await?;

    initialize(&mut server).await?;

    let tools = server
        .request(json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }))
        .await?;
    assert_names(
        &tools["result"]["tools"],
        &[
            "graph_stats",
            "graph_search_nodes",
            "graph_node_details",
            "graph_edge_details",
            "graph_projection",
            "graph_neighbors",
            "graph_shortest_path",
            "graph_file_summary",
            "graph_route_status",
            "fts_search",
            "soul_search",
        ],
    )?;

    let resources = server
        .request(json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "resources/list",
            "params": {}
        }))
        .await?;
    assert_resource_uris(
        &resources["result"]["resources"],
        &[
            "semantic-graph://schema",
            "semantic-graph://workspace",
            "semantic-graph://routes",
            "semantic-graph://local-testbeds",
        ],
    )?;

    let stats = call_tool(&mut server, 4, "graph_stats", json!({})).await?;
    assert_eq!(json!(1), tool_data(&stats)?["workspaceCount"]);
    assert_eq!(json!(3), tool_data(&stats)?["activeNodeCount"]);

    let search = call_tool(
        &mut server,
        5,
        "graph_search_nodes",
        json!({ "query": "run", "limit": 10 }),
    )
    .await?;
    assert_eq!(
        fixture.run_node_id,
        tool_data(&search)?["results"][0]["nodeId"]
    );

    let node = call_tool(
        &mut server,
        6,
        "graph_node_details",
        json!({ "nodeId": fixture.run_node_id }),
    )
    .await?;
    assert_eq!("run", tool_data(&node)?["displayLabel"]);

    let edge = call_tool(
        &mut server,
        7,
        "graph_edge_details",
        json!({ "edgeId": fixture.call_edge_id }),
    )
    .await?;
    assert_eq!("calls", tool_data(&edge)?["relation"]);

    let projection = call_tool(&mut server, 8, "graph_projection", json!({ "limit": 10 })).await?;
    assert_eq!(
        json!(10),
        tool_data(&projection)?["metadata"]["appliedLimit"]
    );

    let neighbors = call_tool(
        &mut server,
        9,
        "graph_neighbors",
        json!({
            "nodeId": fixture.run_node_id,
            "direction": "outgoing",
            "relation": "calls",
            "limit": 10
        }),
    )
    .await?;
    assert_eq!(
        fixture.helper_node_id,
        tool_data(&neighbors)?["outgoing"][0]["adjacentNode"]["nodeId"]
    );

    let path = call_tool(
        &mut server,
        10,
        "graph_shortest_path",
        json!({
            "sourceNodeId": fixture.file_node_id,
            "targetNodeId": fixture.helper_node_id,
            "maxDepth": 4,
            "maxVisitedNodes": 50
        }),
    )
    .await?;
    assert_eq!(json!(true), tool_data(&path)?["found"]);

    let file_summary = call_tool(
        &mut server,
        11,
        "graph_file_summary",
        json!({
            "workspaceId": fixture.workspace_id,
            "filePath": "src/lib.rs",
            "edgeLimit": 10
        }),
    )
    .await?;
    assert_eq!("src/lib.rs", tool_data(&file_summary)?["file"]["path"]);

    let route_status = call_tool(
        &mut server,
        12,
        "graph_route_status",
        json!({
            "workspaceId": fixture.workspace_id,
            "route": "rust.document_symbols",
            "limit": 10
        }),
    )
    .await?;
    assert_eq!(
        "rust.document_symbols",
        tool_data(&route_status)?["statuses"][0]["route"]
    );

    let fts = call_tool(
        &mut server,
        13,
        "fts_search",
        json!({
            "query": "NeedleToken",
            "limit": 10,
            "contextLines": 1
        }),
    )
    .await?;
    assert_eq!("src/lib.rs", tool_data(&fts)?["hits"][0]["path"]);
    assert!(
        tool_data(&fts)?["hits"][0]["snippets"][0]["text"]
            .as_str()
            .ok_or("snippet text should be a string")?
            .contains("NeedleToken")
    );

    read_resource(&mut server, 14, "semantic-graph://schema", "nodes").await?;
    read_resource(&mut server, 15, "semantic-graph://workspace", "read-only").await?;
    read_resource(
        &mut server,
        16,
        "semantic-graph://routes",
        "rust.document_symbols",
    )
    .await?;
    read_resource(
        &mut server,
        17,
        "semantic-graph://local-testbeds",
        "prior art",
    )
    .await?;

    let bad_args = call_tool_error(&mut server, 18, "graph_node_details", json!({})).await?;
    assert_eq!(json!(-32602), bad_args["error"]["code"]);

    let unknown_tool = call_tool_error(&mut server, 19, "graph_missing", json!({})).await?;
    assert_eq!(json!(-32602), unknown_tool["error"]["code"]);

    let unknown_resource = server
        .request(json!({
            "jsonrpc": "2.0",
            "id": 20,
            "method": "resources/read",
            "params": { "uri": "semantic-graph://missing" }
        }))
        .await?;
    assert_eq!(json!(-32002), unknown_resource["error"]["code"]);

    server.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn stdio_server_database_path_override_bypasses_config() -> Result<(), Box<dyn Error>> {
    let fixture = Fixture::seed("override").await?;
    write_config(&fixture.root, &fixture.root.join("wrong.db"))?;
    let database_path = fixture.database_path.display().to_string();
    let mut server = StdioServer::start(
        &fixture.root,
        vec!["--database-path".to_string(), database_path],
    )
    .await?;

    initialize(&mut server).await?;
    let stats = call_tool(&mut server, 20, "graph_stats", json!({})).await?;

    assert_eq!(json!(1), tool_data(&stats)?["workspaceCount"]);
    let fts = call_tool(
        &mut server,
        21,
        "fts_search",
        json!({ "query": "NeedleToken", "limit": 10 }),
    )
    .await?;
    assert_eq!("src/lib.rs", tool_data(&fts)?["hits"][0]["path"]);
    server.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn stdio_server_exposes_soul_search_without_fts_or_live_soul() -> Result<(), Box<dyn Error>> {
    let fixture = SoulFixture::seed("soul-search").await?;
    write_config(&fixture.root, &fixture.database_path)?;
    let mut server = StdioServer::start(&fixture.root, Vec::new()).await?;

    initialize(&mut server).await?;
    let search = call_tool(
        &mut server,
        25,
        "soul_search",
        json!({
            "workspaceId": fixture.workspace_id,
            "query": "feature.checkout",
            "includeMarkdownSources": true,
            "limit": 200
        }),
    )
    .await?;

    let data = tool_data(&search)?;
    assert_eq!(json!(200), data["appliedLimit"]);
    assert_eq!(json!(1), data["totalResults"]);
    assert!(data["nextCursor"].is_null());
    assert_eq!("feature.checkout", data["results"][0]["soulId"]);
    assert_eq!(json!(true), data["results"][0]["hasDocument"]);
    assert_eq!(
        "docs/checkout.md",
        data["results"][0]["document"]["sourceFilePath"]
    );
    assert_eq!(json!(2), data["results"][0]["sourceAnnotationCount"]);
    assert_eq!(json!(2), data["results"][0]["linkedSourceAnnotationCount"]);
    assert_eq!(json!(1), data["results"][0]["markdownReferenceCount"]);
    assert!(
        data["results"][0]["sourceAnnotations"]
            .as_array()
            .ok_or("source annotations should be an array")?
            .iter()
            .any(|source| source["sourceFileLanguage"] == "rust"
                && source["source"]["sourceFilePath"] == "src/backend.rs"
                && source["edge"].is_object())
    );
    assert!(
        data["results"][0]["sourceAnnotations"]
            .as_array()
            .ok_or("source annotations should be an array")?
            .iter()
            .any(|source| source["sourceFileLanguage"] == "csharp"
                && source["source"]["sourceFilePath"] == "frontend/Checkout.cs"
                && source["edge"].is_object())
    );

    let concise = call_tool(
        &mut server,
        26,
        "soul_search",
        json!({
            "workspaceId": fixture.workspace_id,
            "includeMarkdownSources": false,
            "limit": 200
        }),
    )
    .await?;
    let concise_data = tool_data(&concise)?;
    assert_eq!(json!(1), concise_data["totalResults"]);
    assert_eq!("feature.checkout", concise_data["results"][0]["soulId"]);
    assert_eq!(
        json!(2),
        concise_data["results"][0]["sourceAnnotationCount"]
    );
    assert_eq!(
        json!(2),
        concise_data["results"][0]["linkedSourceAnnotationCount"]
    );
    assert_eq!(
        0,
        concise_data["results"][0]["sourceAnnotations"]
            .as_array()
            .ok_or("source annotations should be an array")?
            .len()
    );

    server.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn stdio_server_reports_missing_fts_setup_without_breaking_graph_tools()
-> Result<(), Box<dyn Error>> {
    let fixture = Fixture::seed_graph_only("missing-fts").await?;
    write_config(&fixture.root, &fixture.database_path)?;
    let mut server = StdioServer::start(&fixture.root, Vec::new()).await?;

    initialize(&mut server).await?;
    let stats = call_tool(&mut server, 30, "graph_stats", json!({})).await?;
    assert_eq!(json!(1), tool_data(&stats)?["workspaceCount"]);

    let fts_error = call_tool_error(
        &mut server,
        31,
        "fts_search",
        json!({ "query": "NeedleToken" }),
    )
    .await?;
    assert_eq!(json!(-32602), fts_error["error"]["code"]);
    assert!(
        fts_error["error"]["message"]
            .as_str()
            .ok_or("error message should be a string")?
            .contains("FTS Tantivy index not found")
    );

    server.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn stdio_server_returns_parse_error_for_malformed_json() -> Result<(), Box<dyn Error>> {
    let fixture = Fixture::seed("malformed").await?;
    let database_path = fixture.database_path.display().to_string();
    let mut server = StdioServer::start(
        &fixture.root,
        vec!["--database-path".to_string(), database_path],
    )
    .await?;

    server.write_raw("{not-json\n").await?;
    let response = server.read_next_response().await?;

    assert_eq!(json!(-32700), response["error"]["code"]);
    server.shutdown().await?;
    Ok(())
}

async fn initialize(server: &mut StdioServer) -> Result<(), Box<dyn Error>> {
    let response = server
        .request(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "semantic-graph-mcp-server-test",
                    "version": "0.1.0"
                }
            }
        }))
        .await?;

    assert_eq!(
        json!("semantic-graph-mcp-server"),
        response["result"]["serverInfo"]["name"]
    );
    server
        .notify(json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }))
        .await?;
    Ok(())
}

async fn call_tool(
    server: &mut StdioServer,
    id: i64,
    name: &str,
    arguments: Value,
) -> Result<Value, Box<dyn Error>> {
    server
        .request(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {
                "name": name,
                "arguments": arguments
            }
        }))
        .await
}

async fn call_tool_error(
    server: &mut StdioServer,
    id: i64,
    name: &str,
    arguments: Value,
) -> Result<Value, Box<dyn Error>> {
    let response = call_tool(server, id, name, arguments).await?;
    if response.get("error").is_none() {
        return Err(format!("expected tool error response for {name}: {response}").into());
    }
    Ok(response)
}

async fn read_resource(
    server: &mut StdioServer,
    id: i64,
    uri: &str,
    expected_text: &str,
) -> Result<(), Box<dyn Error>> {
    let response = server
        .request(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "resources/read",
            "params": { "uri": uri }
        }))
        .await?;
    let text = response["result"]["contents"][0]["text"]
        .as_str()
        .ok_or("resource text should be a string")?;

    assert!(
        text.contains(expected_text),
        "resource {uri} did not contain {expected_text}: {text}"
    );
    Ok(())
}

fn tool_data(response: &Value) -> Result<&Value, Box<dyn Error>> {
    response
        .pointer("/result/structuredContent/data")
        .ok_or_else(|| format!("missing structured tool data: {response}").into())
}

fn assert_names(value: &Value, expected: &[&str]) -> Result<(), Box<dyn Error>> {
    let actual = value
        .as_array()
        .ok_or("tools should be an array")?
        .iter()
        .map(|tool| {
            tool["name"]
                .as_str()
                .map(str::to_string)
                .ok_or("tool name should be a string")
        })
        .collect::<Result<Vec<_>, _>>()?;

    assert_eq!(
        expected
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>(),
        actual
    );
    Ok(())
}

fn assert_resource_uris(value: &Value, expected: &[&str]) -> Result<(), Box<dyn Error>> {
    let actual = value
        .as_array()
        .ok_or("resources should be an array")?
        .iter()
        .map(|resource| {
            resource["uri"]
                .as_str()
                .map(str::to_string)
                .ok_or("resource uri should be a string")
        })
        .collect::<Result<Vec<_>, _>>()?;

    assert_eq!(
        expected
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>(),
        actual
    );
    Ok(())
}

struct StdioServer {
    child: Child,
    stdin: ChildStdin,
    stdout: Lines<BufReader<ChildStdout>>,
}

impl StdioServer {
    async fn start(cwd: &Path, args: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let binary = server_binary()?;
        let mut child = Command::new(binary)
            .args(args)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdin = child.stdin.take().ok_or("server stdin should be piped")?;
        let stdout = child.stdout.take().ok_or("server stdout should be piped")?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout).lines(),
        })
    }

    async fn request(&mut self, message: Value) -> Result<Value, Box<dyn Error>> {
        let id = message["id"].clone();
        self.write_json(message).await?;

        loop {
            let response = self.read_next_response().await?;
            if response.get("id") == Some(&id) {
                return Ok(response);
            }
        }
    }

    async fn notify(&mut self, message: Value) -> Result<(), Box<dyn Error>> {
        self.write_json(message).await
    }

    async fn write_json(&mut self, message: Value) -> Result<(), Box<dyn Error>> {
        self.write_raw(&format!("{message}\n")).await
    }

    async fn write_raw(&mut self, message: &str) -> Result<(), Box<dyn Error>> {
        self.stdin.write_all(message.as_bytes()).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    async fn read_next_response(&mut self) -> Result<Value, Box<dyn Error>> {
        let line = timeout(Duration::from_secs(10), self.stdout.next_line()).await??;
        let line = line.ok_or("server stdout closed before response")?;
        Ok(serde_json::from_str(&line)?)
    }

    async fn shutdown(mut self) -> Result<(), Box<dyn Error>> {
        self.child.start_kill()?;
        let _status = self.child.wait().await?;
        Ok(())
    }
}

impl Drop for StdioServer {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

fn server_binary() -> Result<PathBuf, Box<dyn Error>> {
    if let Some(path) = option_env!("CARGO_BIN_EXE_semantic-graph-mcp-server") {
        return Ok(PathBuf::from(path));
    }

    let current_exe = std::env::current_exe()?;
    let deps_dir = current_exe
        .parent()
        .ok_or("test executable should have parent directory")?;
    let debug_dir = deps_dir
        .parent()
        .ok_or("deps directory should have parent directory")?;
    Ok(debug_dir.join("semantic-graph-mcp-server"))
}

fn write_config(root: &Path, database_path: &Path) -> Result<(), Box<dyn Error>> {
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    fs::write(
        config_dir.join("config.toml"),
        format!("[database]\npath = \"{}\"\n", toml_escape(database_path)),
    )?;
    Ok(())
}

fn toml_escape(path: &Path) -> String {
    path.display().to_string().replace('\\', "\\\\")
}

struct Fixture {
    root: PathBuf,
    database_path: PathBuf,
    workspace_id: i64,
    file_node_id: String,
    run_node_id: String,
    helper_node_id: String,
    call_edge_id: String,
}

struct SoulFixture {
    root: PathBuf,
    database_path: PathBuf,
    workspace_id: i64,
}

impl SoulFixture {
    async fn seed(name: &str) -> Result<Self, Box<dyn Error>> {
        let root = temp_dir(name)?;
        let database_path = root.join("semantic-graph.db");
        let writer = WriteManager::start(&database_path).await?;
        writer.migrate().await?;
        let workspace_id = writer
            .create_workspace("file:///mcp-soul-fixture", "soul")
            .await?;
        let run_id = writer
            .start_run(workspace_id, "soul-lsp", Some("fixture"), None)
            .await?;
        let doc_file_id = writer
            .upsert_file(FileInput {
                workspace_id,
                uri: "file:///mcp-soul-fixture/docs/checkout.md",
                path: "docs/checkout.md",
                language: "soul",
                content_hash: Some("doc-hash"),
                last_seen_run_id: Some(run_id),
                properties_json: json!({}),
            })
            .await?;
        let related_file_id = writer
            .upsert_file(FileInput {
                workspace_id,
                uri: "file:///mcp-soul-fixture/docs/related.md",
                path: "docs/related.md",
                language: "soul",
                content_hash: Some("related-hash"),
                last_seen_run_id: Some(run_id),
                properties_json: json!({}),
            })
            .await?;
        let rust_file_id = writer
            .upsert_file(FileInput {
                workspace_id,
                uri: "file:///mcp-soul-fixture/src/backend.rs",
                path: "src/backend.rs",
                language: "soul",
                content_hash: Some("rust-hash"),
                last_seen_run_id: Some(run_id),
                properties_json: json!({}),
            })
            .await?;
        let csharp_file_id = writer
            .upsert_file(FileInput {
                workspace_id,
                uri: "file:///mcp-soul-fixture/frontend/Checkout.cs",
                path: "frontend/Checkout.cs",
                language: "soul",
                content_hash: Some("csharp-hash"),
                last_seen_run_id: Some(run_id),
                properties_json: json!({}),
            })
            .await?;

        let doc_node_id = upsert_soul_node(SoulNodeInput {
            writer: &writer,
            workspace_id,
            run_id,
            file_id: doc_file_id,
            kind: "file",
            name: "feature.checkout",
            symbol_key: "soul-doc:feature.checkout",
            range: range(0, 0, 0, 0),
        })
        .await?;
        let related_node_id = upsert_soul_node(SoulNodeInput {
            writer: &writer,
            workspace_id,
            run_id,
            file_id: related_file_id,
            kind: "string",
            name: "feature.checkout",
            symbol_key: "soul-reference:feature.related-to-checkout",
            range: range(5, 12, 5, 32),
        })
        .await?;
        let rust_node_id = upsert_soul_node(SoulNodeInput {
            writer: &writer,
            workspace_id,
            run_id,
            file_id: rust_file_id,
            kind: "object",
            name: "feature.checkout",
            symbol_key: "soul-annotation:rust:feature.checkout",
            range: range(10, 0, 10, 30),
        })
        .await?;
        let csharp_node_id = upsert_soul_node(SoulNodeInput {
            writer: &writer,
            workspace_id,
            run_id,
            file_id: csharp_file_id,
            kind: "object",
            name: "feature.checkout",
            symbol_key: "soul-annotation:csharp:feature.checkout",
            range: range(20, 0, 20, 30),
        })
        .await?;

        upsert_soul_reference(
            &writer,
            workspace_id,
            run_id,
            rust_file_id,
            &rust_node_id,
            &doc_node_id,
            range(10, 0, 10, 30),
        )
        .await?;
        upsert_soul_reference(
            &writer,
            workspace_id,
            run_id,
            csharp_file_id,
            &csharp_node_id,
            &doc_node_id,
            range(20, 0, 20, 30),
        )
        .await?;
        upsert_soul_reference(
            &writer,
            workspace_id,
            run_id,
            related_file_id,
            &related_node_id,
            &doc_node_id,
            range(5, 12, 5, 32),
        )
        .await?;

        writer.finish_run(run_id, "complete").await?;
        writer.shutdown().await?;

        Ok(Self {
            root,
            database_path,
            workspace_id,
        })
    }
}

struct SoulNodeInput<'a> {
    writer: &'a WriteHandle,
    workspace_id: i64,
    run_id: i64,
    file_id: i64,
    kind: &'a str,
    name: &'a str,
    symbol_key: &'a str,
    range: TextRange,
}

async fn upsert_soul_node(input: SoulNodeInput<'_>) -> Result<String, Box<dyn Error>> {
    Ok(input
        .writer
        .upsert_node(NodeInput {
            workspace_id: input.workspace_id,
            language: "soul",
            kind: input.kind,
            name: input.name,
            qualified_name: Some(input.name),
            display_name: Some(input.name),
            symbol_key: input.symbol_key,
            file_id: Some(input.file_id),
            range: Some(input.range),
            selection_range: Some(input.range),
            container_node_id: None,
            properties_json: json!({}),
            run_id: Some(input.run_id),
        })
        .await?)
}

async fn upsert_soul_reference(
    writer: &WriteHandle,
    workspace_id: i64,
    run_id: i64,
    file_id: i64,
    source_node_id: &str,
    target_node_id: &str,
    range: TextRange,
) -> Result<(), Box<dyn Error>> {
    let edge_id = writer
        .upsert_edge(EdgeInput {
            workspace_id,
            src_node_id: source_node_id,
            dst_node_id: target_node_id,
            relation: "references",
            context: Some("symbol"),
            confidence: "EXTRACTED",
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({ "source_resolution": "symbol" }),
            run_id: Some(run_id),
        })
        .await?;
    writer
        .insert_edge_evidence(EdgeEvidenceInput {
            edge_id: &edge_id,
            run_id,
            provider: "soul-lsp",
            lsp_method: Some("textDocument/references"),
            file_id: Some(file_id),
            range: Some(range),
            raw_json: Some(json!({ "source": "soul-fixture" })),
        })
        .await?;
    Ok(())
}

impl Fixture {
    async fn seed(name: &str) -> Result<Self, Box<dyn Error>> {
        Self::seed_with_fts(name, true).await
    }

    async fn seed_graph_only(name: &str) -> Result<Self, Box<dyn Error>> {
        Self::seed_with_fts(name, false).await
    }

    async fn seed_with_fts(name: &str, include_fts: bool) -> Result<Self, Box<dyn Error>> {
        let root = temp_dir(name)?;
        let database_path = root.join("semantic-graph.db");
        let fts_index_path = database_path.with_extension("tantivy");
        let writer = WriteManager::start(&database_path).await?;
        writer.migrate().await?;
        let workspace_id = writer
            .create_workspace("file:///mcp-fixture", "rust")
            .await?;
        let run_id = writer
            .start_run(workspace_id, "rust-analyzer", Some("fixture"), None)
            .await?;
        let file_id = writer
            .upsert_file(FileInput {
                workspace_id,
                uri: "file:///mcp-fixture/src/lib.rs",
                path: "src/lib.rs",
                language: "rust",
                content_hash: Some("lib-hash"),
                last_seen_run_id: Some(run_id),
                properties_json: json!({ "fixture": true }),
            })
            .await?;

        let file_node_id = node_id(workspace_id, "rust", "file:///mcp-fixture/src/lib.rs");
        let run_node_id = node_id(workspace_id, "rust", "function:fixture::run");
        let helper_node_id = node_id(workspace_id, "rust", "function:fixture::helper");
        seed_nodes(
            &writer,
            workspace_id,
            run_id,
            file_id,
            &file_node_id,
            &run_node_id,
            &helper_node_id,
        )
        .await?;
        let call_edge_id = seed_edges(
            &writer,
            workspace_id,
            run_id,
            &file_node_id,
            &run_node_id,
            &helper_node_id,
        )
        .await?;
        seed_occurrences(&writer, run_id, file_id, &run_node_id, &helper_node_id).await?;
        seed_edge_evidence(&writer, run_id, file_id, &call_edge_id).await?;
        seed_routes(
            &writer,
            workspace_id,
            run_id,
            file_id,
            &run_node_id,
            &call_edge_id,
        )
        .await?;
        if include_fts {
            seed_fts_documents(&writer, workspace_id, run_id, &fts_index_path).await?;
        }
        writer.finish_run(run_id, "complete").await?;
        writer.shutdown().await?;

        Ok(Self {
            root,
            database_path,
            workspace_id,
            file_node_id,
            run_node_id,
            helper_node_id,
            call_edge_id,
        })
    }
}

async fn seed_fts_documents(
    writer: &WriteHandle,
    workspace_id: i64,
    run_id: i64,
    fts_index_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let content = "fn run() {\n    let token = \"NeedleToken\";\n    helper();\n}\n";
    let documents = vec![FtsWriteBatchDocumentInput {
        workspace_id,
        uri: "file:///mcp-fixture/src/lib.rs".to_string(),
        path: "src/lib.rs".to_string(),
        language: "rust".to_string(),
        content_hash: "fts-lib-hash".to_string(),
        byte_len: content.len() as i64,
        run_id,
        content: content.to_string(),
        properties_json: json!({ "route": "fts.full_text" }),
    }];
    writer
        .write_fts_batch(FtsWriteBatchInput {
            documents: documents.clone(),
            seen_documents: Vec::new(),
        })
        .await?;

    let index = TantivyFtsIndex::open_or_create(fts_index_path)?;
    index.apply_update(TantivyFtsIndexUpdate {
        documents: documents
            .into_iter()
            .map(|document| TantivyFtsDocument {
                uri: document.uri,
                path: document.path,
                language: document.language,
                content_hash: document.content_hash,
                content: document.content,
            })
            .collect(),
        deleted_uris: Vec::new(),
        indexing_workers: 1,
    })?;

    Ok(())
}

async fn seed_nodes(
    writer: &WriteHandle,
    workspace_id: i64,
    run_id: i64,
    file_id: i64,
    file_node_id: &str,
    run_node_id: &str,
    helper_node_id: &str,
) -> Result<(), Box<dyn Error>> {
    writer
        .upsert_node(NodeInput {
            workspace_id,
            language: "rust",
            kind: "file",
            name: "lib.rs",
            qualified_name: Some("src/lib.rs"),
            display_name: Some("lib.rs"),
            symbol_key: "file:///mcp-fixture/src/lib.rs",
            file_id: Some(file_id),
            range: None,
            selection_range: None,
            container_node_id: None,
            properties_json: json!({}),
            run_id: Some(run_id),
        })
        .await?;
    writer
        .upsert_node(NodeInput {
            workspace_id,
            language: "rust",
            kind: "function",
            name: "run",
            qualified_name: Some("fixture::run"),
            display_name: Some("run"),
            symbol_key: "function:fixture::run",
            file_id: Some(file_id),
            range: Some(range(1, 0, 4, 1)),
            selection_range: Some(range(1, 3, 1, 6)),
            container_node_id: Some(file_node_id),
            properties_json: json!({}),
            run_id: Some(run_id),
        })
        .await?;
    writer
        .upsert_node(NodeInput {
            workspace_id,
            language: "rust",
            kind: "function",
            name: "helper",
            qualified_name: Some("fixture::helper"),
            display_name: Some("helper"),
            symbol_key: "function:fixture::helper",
            file_id: Some(file_id),
            range: Some(range(6, 0, 8, 1)),
            selection_range: Some(range(6, 3, 6, 9)),
            container_node_id: Some(file_node_id),
            properties_json: json!({}),
            run_id: Some(run_id),
        })
        .await?;

    assert_eq!(
        file_node_id,
        node_id(workspace_id, "rust", "file:///mcp-fixture/src/lib.rs")
    );
    assert_eq!(
        run_node_id,
        node_id(workspace_id, "rust", "function:fixture::run")
    );
    assert_eq!(
        helper_node_id,
        node_id(workspace_id, "rust", "function:fixture::helper")
    );
    Ok(())
}

async fn seed_edges(
    writer: &WriteHandle,
    workspace_id: i64,
    run_id: i64,
    file_node_id: &str,
    run_node_id: &str,
    helper_node_id: &str,
) -> Result<String, Box<dyn Error>> {
    writer
        .upsert_edge(EdgeInput {
            workspace_id,
            src_node_id: file_node_id,
            dst_node_id: run_node_id,
            relation: "contains",
            context: Some("document-symbol"),
            confidence: "EXTRACTED",
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({}),
            run_id: Some(run_id),
        })
        .await?;
    let call_edge_id = writer
        .upsert_edge(EdgeInput {
            workspace_id,
            src_node_id: run_node_id,
            dst_node_id: helper_node_id,
            relation: "calls",
            context: Some("direct"),
            confidence: "EXTRACTED",
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({}),
            run_id: Some(run_id),
        })
        .await?;

    assert_eq!(
        call_edge_id,
        edge_id(
            workspace_id,
            run_node_id,
            helper_node_id,
            "calls",
            Some("direct")
        )
    );
    Ok(call_edge_id)
}

async fn seed_occurrences(
    writer: &WriteHandle,
    run_id: i64,
    file_id: i64,
    run_node_id: &str,
    helper_node_id: &str,
) -> Result<(), Box<dyn Error>> {
    writer
        .insert_occurrence(OccurrenceInput {
            node_id: run_node_id,
            run_id,
            file_id,
            role: "definition",
            range: range(1, 0, 4, 1),
            enclosing_node_id: None,
            raw_json: Some(json!({ "source": "documentSymbol" })),
        })
        .await?;
    writer
        .insert_occurrence(OccurrenceInput {
            node_id: helper_node_id,
            run_id,
            file_id,
            role: "call",
            range: range(2, 4, 2, 12),
            enclosing_node_id: Some(run_node_id),
            raw_json: Some(json!({ "source": "callHierarchy" })),
        })
        .await?;
    Ok(())
}

async fn seed_edge_evidence(
    writer: &WriteHandle,
    run_id: i64,
    file_id: i64,
    call_edge_id: &str,
) -> Result<(), Box<dyn Error>> {
    writer
        .insert_edge_evidence(EdgeEvidenceInput {
            edge_id: call_edge_id,
            run_id,
            provider: "rust-analyzer",
            lsp_method: Some("callHierarchy/outgoingCalls"),
            file_id: Some(file_id),
            range: Some(range(2, 4, 2, 12)),
            raw_json: Some(json!({ "source": "call-evidence" })),
        })
        .await?;
    Ok(())
}

async fn seed_routes(
    writer: &WriteHandle,
    workspace_id: i64,
    run_id: i64,
    file_id: i64,
    run_node_id: &str,
    call_edge_id: &str,
) -> Result<(), Box<dyn Error>> {
    writer
        .start_route_status(RouteStatusStartInput {
            workspace_id,
            route: "rust.document_symbols",
            scope: "file",
            scope_key: "file:///mcp-fixture/src/lib.rs",
            file_id: Some(file_id),
            provider: "rust-analyzer",
            provider_version: Some("fixture"),
            content_hash: Some("lib-hash"),
            run_id,
            diagnostics_json: json!({ "phase": "started" }),
        })
        .await?;
    writer
        .complete_route_status(RouteStatusCompleteInput {
            workspace_id,
            route: "rust.document_symbols",
            scope: "file",
            scope_key: "file:///mcp-fixture/src/lib.rs",
            provider: "rust-analyzer",
            provider_version: Some("fixture"),
            content_hash: Some("lib-hash"),
            run_id,
            diagnostics_json: json!({ "file": "clean" }),
        })
        .await?;
    writer
        .start_route_status(RouteStatusStartInput {
            workspace_id,
            route: "rust.references",
            scope: "workspace",
            scope_key: "file:///mcp-fixture",
            file_id: None,
            provider: "rust-analyzer",
            provider_version: Some("fixture"),
            content_hash: None,
            run_id,
            diagnostics_json: json!({ "phase": "started" }),
        })
        .await?;
    writer
        .complete_route_status(RouteStatusCompleteInput {
            workspace_id,
            route: "rust.references",
            scope: "workspace",
            scope_key: "file:///mcp-fixture",
            provider: "rust-analyzer",
            provider_version: Some("fixture"),
            content_hash: None,
            run_id,
            diagnostics_json: json!({ "workspace": "clean" }),
        })
        .await?;
    writer
        .record_route_observation(RouteObservationInput {
            workspace_id,
            run_id,
            route: "rust.document_symbols",
            scope: "file",
            scope_key: "file:///mcp-fixture/src/lib.rs",
            provider: "rust-analyzer",
            entity_kind: "node",
            entity_id: run_node_id,
            source_file_id: Some(file_id),
            properties_json: json!({ "observed": "node" }),
        })
        .await?;
    writer
        .record_route_observation(RouteObservationInput {
            workspace_id,
            run_id,
            route: "rust.references",
            scope: "workspace",
            scope_key: "file:///mcp-fixture",
            provider: "rust-analyzer",
            entity_kind: "edge",
            entity_id: call_edge_id,
            source_file_id: Some(file_id),
            properties_json: json!({ "observed": "edge" }),
        })
        .await?;
    Ok(())
}

fn range(start_line: i64, start_col: i64, end_line: i64, end_col: i64) -> TextRange {
    TextRange {
        start_line,
        start_col,
        end_line,
        end_col,
    }
}

fn temp_dir(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let root =
        std::env::temp_dir().join(format!("semantic-graph-mcp-stdio-{name}-{nanos}-{counter}"));
    fs::create_dir_all(&root)?;
    Ok(root)
}
