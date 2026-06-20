use codebase_memory_mcp::mcp::{McpServer, SERVER_NAME};
use rmcp::{model::CallToolRequestParams, ServerHandler, ServiceExt};

fn assert_server_handler<T: ServerHandler>() {}

#[test]
fn cbm_uses_official_rmcp_server_handler() {
    assert_server_handler::<McpServer>();
    assert_eq!(SERVER_NAME, "codebase-memory-mcp");
}

#[tokio::test]
async fn official_client_lists_calls_and_validates_tools() {
    std::env::set_var("CBM_WATCHER", "0");
    std::env::set_var("CBRLM_WATCHER", "0");
    let (server_transport, client_transport) = tokio::io::duplex(1024 * 1024);

    let server_task = tokio::spawn(async move {
        McpServer::new()
            .serve(server_transport)
            .await
            .expect("start rmcp server")
            .waiting()
            .await
            .expect("wait rmcp server");
    });

    let client = ().serve(client_transport).await.expect("start rmcp client");
    let tools = client.list_all_tools().await.expect("list tools");
    assert_eq!(tools.len(), 14);
    assert!(tools.iter().any(|tool| tool.name == "index_repository"));
    assert!(!tools.iter().any(|tool| tool.name.starts_with("rlm_")));

    let result = client
        .call_tool(CallToolRequestParams::new("list_projects"))
        .await
        .expect("call list_projects");
    assert_eq!(result.is_error, Some(false));
    assert!(result.content[0]
        .raw
        .as_text()
        .is_some_and(|content| content.text.contains("projects")));

    let invalid = client
        .call_tool(CallToolRequestParams::new("search_graph"))
        .await;
    assert!(
        invalid.is_err(),
        "missing required project must be invalid params"
    );

    client.cancel().await.expect("cancel client");
    server_task.await.expect("join server task");
}
