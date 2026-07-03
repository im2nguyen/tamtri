use serde_json::json;
use tamtri_core::mcp::{McpClient, McpClientConfig};

#[tokio::test]
async fn integration_echo_tool() {
    let command = env!("CARGO_BIN_EXE_mock-mcp-server");
    let client = McpClient::connect_stdio(command, &[], &[], McpClientConfig::default())
        .await
        .unwrap();

    let tools = client.list_tools().await.unwrap();
    assert_eq!(tools.len(), 5);
    assert!(tools.iter().any(|tool| tool.name == "echo"));

    let result = client
        .call_tool("echo", json!({"message": "hello"}), None)
        .await
        .unwrap();
    assert_eq!(result.is_error, Some(false));
    assert_eq!(
        result.structured_content.unwrap()["echo"]["message"],
        "hello"
    );

    let resources = client.list_resources().await.unwrap();
    assert_eq!(resources[0].uri, "mock://report");
    let resource = client.read_resource("mock://report").await.unwrap();
    assert_eq!(resource.contents[0]["text"], "mock resource");

    let prompts = client.list_prompts().await.unwrap();
    assert_eq!(prompts[0].name, "summarize");
    let prompt = client
        .get_prompt("summarize", json!({"topic": "tamtri"}))
        .await
        .unwrap();
    assert_eq!(prompt.messages[0]["role"], "user");

    client.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires npx and network; spawns @modelcontextprotocol/server-everything"]
async fn integration_server_everything() {
    let client = McpClient::connect_stdio(
        "npx",
        &[
            "-y".to_string(),
            "@modelcontextprotocol/server-everything".to_string(),
        ],
        &[],
        McpClientConfig::default(),
    )
    .await
    .unwrap();

    let tools = client.list_tools().await.unwrap();
    assert!(!tools.is_empty());

    client.close().await.unwrap();
}
