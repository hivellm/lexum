//! MCP Service implementation for Lexum

use crate::handlers::index::AppState;
use crate::protocols::mcp::handlers::handle_mcp_tool;
use crate::protocols::mcp::tools::get_mcp_tools;
use rmcp::ServerHandler;
use rmcp::model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo};
use std::sync::Arc;

/// Lexum MCP Service handler
pub struct LexumMcpService {
    #[allow(missing_docs)]
    pub state: Arc<AppState>,
}

impl ServerHandler for LexumMcpService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation {
                name: "lexum-server".to_string(),
                title: Some("Lexum Search Engine Server".to_string()),
                version: env!("CARGO_PKG_VERSION").to_string(),
                website_url: Some("https://github.com/hivellm/lexum".to_string()),
                icons: None,
            },
            instructions: Some(
                "Lexum - High-performance Elasticsearch-compatible search engine with MCP + REST API.".to_string(),
            ),
        }
    }

    #[allow(clippy::manual_async_fn)]
    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl std::future::Future<
        Output = Result<rmcp::model::ListToolsResult, rmcp::model::ErrorData>,
    > + Send
    + '_ {
        async move {
            use rmcp::model::ListToolsResult;

            let tools = get_mcp_tools();

            Ok(ListToolsResult {
                tools,
                next_cursor: None,
            })
        }
    }

    #[allow(clippy::manual_async_fn)]
    fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParam,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl std::future::Future<
        Output = Result<rmcp::model::CallToolResult, rmcp::model::ErrorData>,
    > + Send
    + '_ {
        async move { handle_mcp_tool(request, self.state.clone()).await }
    }

    #[allow(clippy::manual_async_fn)]
    fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl std::future::Future<
        Output = Result<rmcp::model::ListResourcesResult, rmcp::model::ErrorData>,
    > + Send
    + '_ {
        async move {
            use rmcp::model::ListResourcesResult;
            Ok(ListResourcesResult {
                resources: vec![],
                next_cursor: None,
            })
        }
    }
}
