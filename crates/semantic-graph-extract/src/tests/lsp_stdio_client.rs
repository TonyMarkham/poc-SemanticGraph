use std::error::Error;

use crate::error::{ExtractError, ExtractResult};
use crate::lsp_stdio::read_framed_json;
use serde_json::Value;
use tokio::io::{AsyncWriteExt, BufReader, duplex};

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

async fn read_one(bytes: &[u8]) -> ExtractResult<Value> {
    let (mut writer, reader) = duplex(512);
    writer
        .write_all(bytes)
        .await
        .map_err(|source| ExtractError::io("write test fixture", None, source))?;
    drop(writer);

    let mut reader = BufReader::new(reader);
    read_framed_json(&mut reader).await
}
