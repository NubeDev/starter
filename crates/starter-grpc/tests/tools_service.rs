//! End-to-end integration tests over a real loopback gRPC channel.
//! Exercises both code paths the consumer cares about: open + bearer.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use starter_spi::auth::{Authenticator, Principal, Role};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_spi::{Error as SpiError, Result as SpiResult};

use starter_grpc::proto::tools_client::ToolsClient;
use starter_grpc::proto::{CallToolRequest, ListToolsRequest};
use starter_grpc::testing::TestServer;
use starter_grpc::{GrpcAuth, ToolRegistry};

// --- helpers ---------------------------------------------------------------

struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "echo".into(),
            description: "Return the input unchanged.".into(),
            input_schema: json!({ "type": "object" }),
        }
    }
    async fn invoke(&self, input: serde_json::Value) -> SpiResult<serde_json::Value> {
        Ok(input)
    }
}

struct FixedAuthenticator {
    expected: &'static str,
}

#[async_trait]
impl Authenticator for FixedAuthenticator {
    async fn verify(&self, credential: &str) -> SpiResult<Principal> {
        if credential == self.expected {
            Ok(Principal {
                subject: "tester".into(),
                role: Role::Admin,
                scopes: Vec::new(),
                extra: Default::default(),
            })
        } else {
            Err(SpiError::Unauthenticated)
        }
    }
}

// --- tests -----------------------------------------------------------------

#[tokio::test]
async fn list_tools_returns_registered_definitions() {
    let registry = Arc::new(ToolRegistry::new().register(EchoTool));
    let server = TestServer::start(registry, GrpcAuth::Open).await;

    let mut client = ToolsClient::connect(server.endpoint()).await.unwrap();
    let resp = client.list_tools(ListToolsRequest {}).await.unwrap();
    let tools = resp.into_inner().tools;
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "echo");
    assert_eq!(tools[0].description, "Return the input unchanged.");
    let schema: serde_json::Value = serde_json::from_str(&tools[0].input_schema_json).unwrap();
    assert_eq!(schema["type"], "object");
}

#[tokio::test]
async fn call_tool_invokes_named_tool() {
    let registry = Arc::new(ToolRegistry::new().register(EchoTool));
    let server = TestServer::start(registry, GrpcAuth::Open).await;

    let mut client = ToolsClient::connect(server.endpoint()).await.unwrap();
    let resp = client
        .call_tool(CallToolRequest {
            name: "echo".into(),
            arguments_json: r#"{"a":1}"#.into(),
        })
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&resp.into_inner().result_json).unwrap();
    assert_eq!(parsed["a"], 1);
}

#[tokio::test]
async fn call_tool_unknown_returns_not_found() {
    let registry = Arc::new(ToolRegistry::new().register(EchoTool));
    let server = TestServer::start(registry, GrpcAuth::Open).await;

    let mut client = ToolsClient::connect(server.endpoint()).await.unwrap();
    let err = client
        .call_tool(CallToolRequest {
            name: "nope".into(),
            arguments_json: "{}".into(),
        })
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn call_tool_invalid_json_returns_invalid_argument() {
    let registry = Arc::new(ToolRegistry::new().register(EchoTool));
    let server = TestServer::start(registry, GrpcAuth::Open).await;

    let mut client = ToolsClient::connect(server.endpoint()).await.unwrap();
    let err = client
        .call_tool(CallToolRequest {
            name: "echo".into(),
            arguments_json: "{not json".into(),
        })
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn bearer_required_rejects_missing_token() {
    let registry = Arc::new(ToolRegistry::new().register(EchoTool));
    let auth = GrpcAuth::Bearer(Arc::new(FixedAuthenticator { expected: "good" }));
    let server = TestServer::start(registry, auth).await;

    let mut client = ToolsClient::connect(server.endpoint()).await.unwrap();
    let err = client.list_tools(ListToolsRequest {}).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
}

#[tokio::test]
#[allow(clippy::result_large_err)] // `tonic::Status` size is fixed by the interceptor signature.
async fn bearer_required_accepts_valid_token() {
    let registry = Arc::new(ToolRegistry::new().register(EchoTool));
    let auth = GrpcAuth::Bearer(Arc::new(FixedAuthenticator { expected: "good" }));
    let server = TestServer::start(registry, auth).await;

    let channel = tonic::transport::Endpoint::from_shared(server.endpoint())
        .unwrap()
        .connect()
        .await
        .unwrap();
    let mut client = ToolsClient::with_interceptor(channel, |mut req: tonic::Request<()>| {
        req.metadata_mut()
            .insert("authorization", "Bearer good".parse().unwrap());
        Ok(req)
    });
    let resp = client.list_tools(ListToolsRequest {}).await.unwrap();
    assert_eq!(resp.into_inner().tools.len(), 1);
}

#[tokio::test]
#[allow(clippy::result_large_err)] // `tonic::Status` size is fixed by the interceptor signature.
async fn bearer_required_rejects_wrong_token() {
    let registry = Arc::new(ToolRegistry::new().register(EchoTool));
    let auth = GrpcAuth::Bearer(Arc::new(FixedAuthenticator { expected: "good" }));
    let server = TestServer::start(registry, auth).await;

    let channel = tonic::transport::Endpoint::from_shared(server.endpoint())
        .unwrap()
        .connect()
        .await
        .unwrap();
    let mut client = ToolsClient::with_interceptor(channel, |mut req: tonic::Request<()>| {
        req.metadata_mut()
            .insert("authorization", "Bearer bad".parse().unwrap());
        Ok(req)
    });
    let err = client.list_tools(ListToolsRequest {}).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
}
