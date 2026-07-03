use serde_json::json;
use tamtri_core::mcp::{McpClient, McpClientConfig};

#[tokio::test]
async fn integration_echo_tool() {
    let command = env!("CARGO_BIN_EXE_mock-mcp-server");
    let client = McpClient::connect_stdio(command, &[], &[], McpClientConfig::default())
        .await
        .unwrap();

    let tools = client.list_tools().await.unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "echo");

    let result = client
        .call_tool("echo", json!({"message": "hello"}))
        .await
        .unwrap();
    assert_eq!(result.is_error, Some(false));
    assert_eq!(
        result.structured_content.unwrap()["echo"]["message"],
        "hello"
    );

    client.close().await.unwrap();
}
