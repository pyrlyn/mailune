//! Local MCP server.
//!
//! `list_subjects` is read-only. `send` forwards to [`send_mail`], which
//! refuses unless the in-app approval flag is set. This crate does not open
//! a socket; a caller supplies the transport.

use std::sync::Arc;

use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    JsonObject, ListToolsResult, ServerCapabilities, ServerConfig, Tool, ToolAnnotations,
};
use rmcp::service::RequestContext;
use rmcp::{RoleServer, ServerHandler};

/// Why a tool call was refused.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Send was requested without the in-app approval flag.
    #[error("send needs in-app approval")]
    SendNotApproved,
}

/// Read-only export. No mailbox is opened here; the shell fills subjects later.
#[must_use]
pub fn list_subjects() -> &'static str {
    "no subjects"
}

/// Send export. The flag is the in-app approval the shell already collected.
///
/// # Errors
///
/// [`Error::SendNotApproved`] when `approved` is false.
pub fn send_mail(approved: bool) -> Result<&'static str, Error> {
    if !approved {
        return Err(Error::SendNotApproved);
    }
    Ok("accepted")
}

/// Server the shell can `serve` on whatever transport it already has.
#[derive(Debug, Clone, Copy)]
pub struct LocalServer {
    send_approved: bool,
}

impl LocalServer {
    /// `send_approved` is the in-app approval flag for this process.
    #[must_use]
    pub fn new(send_approved: bool) -> Self {
        Self { send_approved }
    }
}

impl ServerHandler for LocalServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("mailune", "0.1.0"))
    }

    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, rmcp::ErrorData> {
        Ok(ListToolsResult::with_all_items(tools()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, rmcp::ErrorData> {
        Ok(answer(&request.name, self.send_approved).into())
    }
}

fn tools() -> Vec<Tool> {
    vec![
        Tool::new("list_subjects", "List message subjects", empty_input())
            .with_annotations(ToolAnnotations::new().read_only(true)),
        Tool::new(
            "send",
            "Send a message after in-app approval",
            empty_input(),
        )
        .with_annotations(ToolAnnotations::new().read_only(false)),
    ]
}

fn empty_input() -> Arc<JsonObject> {
    let mut schema = JsonObject::new();
    schema.insert("type".into(), serde_json::Value::String("object".into()));
    Arc::new(schema)
}

/// Each name forwards one export. An unknown name is a tool error, not a send.
fn answer(name: &str, approved: bool) -> CallToolResult {
    match name {
        "list_subjects" => CallToolResult::success(vec![ContentBlock::text(list_subjects())]),
        "send" => match send_mail(approved) {
            Ok(text) => CallToolResult::success(vec![ContentBlock::text(text)]),
            Err(Error::SendNotApproved) => {
                CallToolResult::error(vec![ContentBlock::text("send needs in-app approval")])
            }
        },
        _ => CallToolResult::error(vec![ContentBlock::text("unknown tool")]),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use rmcp::ServiceExt;
    use rmcp::model::CallToolRequestParams;

    use super::{LocalServer, list_subjects, send_mail};

    async fn call(approved: bool, name: &'static str) -> rmcp::model::CallToolResult {
        let (server_transport, client_transport) = tokio::io::duplex(4096);
        let server_task = tokio::spawn(async move {
            let running = LocalServer::new(approved)
                .serve(server_transport)
                .await
                .map_err(|err| err.to_string())?;
            running.waiting().await.map_err(|err| err.to_string())?;
            Ok::<(), String>(())
        });
        let client = ().serve(client_transport).await.expect("in-memory client");
        let result = client
            .call_tool(CallToolRequestParams::new(name))
            .await
            .expect("tool call");
        client.cancel().await.expect("client cancel");
        tokio::time::timeout(Duration::from_secs(5), server_task)
            .await
            .expect("server finished")
            .expect("server task")
            .expect("server");
        result
    }

    #[tokio::test(flavor = "current_thread")]
    async fn read_only_tool_answers_and_send_needs_approval() {
        let listed = {
            let (server_transport, client_transport) = tokio::io::duplex(4096);
            let server_task = tokio::spawn(async move {
                let running = LocalServer::new(false)
                    .serve(server_transport)
                    .await
                    .map_err(|err| err.to_string())?;
                running.waiting().await.map_err(|err| err.to_string())
            });
            let client = ().serve(client_transport).await.expect("client");
            let tools = client.list_tools(None).await.expect("tools").tools;
            client.cancel().await.expect("cancel");
            tokio::time::timeout(Duration::from_secs(5), server_task)
                .await
                .expect("server finished")
                .expect("join")
                .expect("server");
            tools
        };
        let names: Vec<_> = listed.iter().map(|tool| tool.name.as_ref()).collect();
        assert_eq!(names, ["list_subjects", "send"]);
        assert_eq!(
            listed[0]
                .annotations
                .as_ref()
                .and_then(|a| a.read_only_hint),
            Some(true)
        );

        let read = call(false, "list_subjects").await;
        assert_eq!(read.is_error, Some(false));
        assert_eq!(
            read.content
                .first()
                .and_then(|block| block.as_text())
                .map(|text| text.text.as_str()),
            Some(list_subjects())
        );

        let blocked = call(false, "send").await;
        assert_eq!(blocked.is_error, Some(true));

        let allowed = call(true, "send").await;
        assert_eq!(allowed.is_error, Some(false));
        assert_eq!(
            send_mail(false).unwrap_err().to_string(),
            "send needs in-app approval"
        );
    }
}
