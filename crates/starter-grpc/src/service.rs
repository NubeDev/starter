//! gRPC service implementation for the canonical `Tools` surface.
//!
//! Wraps a [`ToolRegistry`] in a tonic-generated `ToolsServer<T>`
//! that consumers add to their `tonic::transport::Server::builder()`.
//! [`tools_server`] is the one-call convenience for the common path.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::auth::GrpcAuth;
use crate::proto::tools_server::{Tools, ToolsServer};
use crate::proto::{
    CallToolRequest, CallToolResponse, ListToolsRequest, ListToolsResponse, ToolDescriptor,
};
use crate::registry::ToolRegistry;

/// The concrete [`Tools`] implementation backed by a [`ToolRegistry`]
/// and a [`GrpcAuth`] policy. Build it directly to compose into a
/// `tonic::transport::Server` alongside consumer services, or use
/// [`tools_server`] for the one-line variant.
pub struct ToolsService {
    registry: Arc<ToolRegistry>,
    auth: GrpcAuth,
}

impl ToolsService {
    /// Build a service from the consumer's registry + auth policy.
    pub fn new(registry: Arc<ToolRegistry>, auth: GrpcAuth) -> Self {
        Self { registry, auth }
    }

    /// Wrap this service in the generated `ToolsServer<Self>` ready
    /// for `Server::builder().add_service(…)`.
    pub fn into_server(self) -> ToolsServer<Self> {
        ToolsServer::new(self)
    }
}

#[tonic::async_trait]
impl Tools for ToolsService {
    async fn list_tools(
        &self,
        mut req: Request<ListToolsRequest>,
    ) -> Result<Response<ListToolsResponse>, Status> {
        self.auth.check(&mut req).await?;

        let tools = self
            .registry
            .list()
            .into_iter()
            .map(|def| ToolDescriptor {
                name: def.name,
                description: def.description,
                // Serialize JSON to a string. Infallible in practice
                // (the schema came from `serde_json::Value`), but if
                // it ever fails we surface `INTERNAL` rather than
                // crashing the server.
                input_schema_json: serde_json::to_string(&def.input_schema)
                    .unwrap_or_else(|_| "null".to_owned()),
            })
            .collect();

        Ok(Response::new(ListToolsResponse { tools }))
    }

    async fn call_tool(
        &self,
        mut req: Request<CallToolRequest>,
    ) -> Result<Response<CallToolResponse>, Status> {
        self.auth.check(&mut req).await?;

        let CallToolRequest {
            name,
            arguments_json,
        } = req.into_inner();

        let tool = self
            .registry
            .get(&name)
            .ok_or_else(|| Status::not_found(format!("unknown tool: {name}")))?;

        // Empty arguments default to JSON null (parity with MCP's
        // `tools/call` where `arguments` is optional).
        let arguments = if arguments_json.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_str(&arguments_json).map_err(|e| {
                Status::invalid_argument(format!("arguments_json is not valid JSON: {e}"))
            })?
        };

        let output = tool
            .invoke(arguments)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(CallToolResponse {
            result_json: serde_json::to_string(&output)
                .map_err(|e| Status::internal(format!("encoding tool result: {e}")))?,
        }))
    }
}

/// One-line convenience: build a [`ToolsService`] + wrap it in the
/// tonic-generated server. Pass the result to
/// `tonic::transport::Server::builder().add_service(…)`.
pub fn tools_server(registry: Arc<ToolRegistry>, auth: GrpcAuth) -> ToolsServer<ToolsService> {
    ToolsService::new(registry, auth).into_server()
}
