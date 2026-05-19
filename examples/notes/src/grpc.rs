//! gRPC surface. Starter ships no gRPC support — this entire file
//! is consumer-owned, layered on top of starter as if starter were
//! just another library. The same `NoteStore` backs it.
//!
//! Auth is enforced by checking the `authorization` metadata header
//! against the same `Authenticator` impl the REST surface uses.
//! Sharing the auth check across surfaces falls out for free because
//! `Authenticator::verify(&str)` is transport-agnostic.

use std::sync::Arc;

use starter_spi::auth::Authenticator;
use tonic::{Request, Response, Status};

use crate::domain::NoteStore;

pub mod proto {
    tonic::include_proto!("notes.v1");
}

use proto::note_service_server::{NoteService, NoteServiceServer};
use proto::{GetRequest, ListRequest, ListResponse, Note as ProtoNote};

pub struct NotesGrpc {
    pub store: Arc<NoteStore>,
    pub authenticator: Arc<dyn Authenticator>,
}

impl NotesGrpc {
    pub fn into_server(self) -> NoteServiceServer<Self> {
        NoteServiceServer::new(self)
    }

    async fn check_auth<T>(&self, req: &Request<T>) -> Result<(), Status> {
        let token = req
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .ok_or_else(|| Status::unauthenticated("missing bearer"))?;
        match self.authenticator.verify(token).await {
            Ok(_principal) => Ok(()),
            Err(_) => Err(Status::unauthenticated("invalid bearer")),
        }
    }
}

#[tonic::async_trait]
impl NoteService for NotesGrpc {
    async fn get(&self, req: Request<GetRequest>) -> Result<Response<ProtoNote>, Status> {
        self.check_auth(&req).await?;
        let id = req.into_inner().id;
        let note = self
            .store
            .get(&id)
            .await
            .map_err(|e| match e {
                crate::domain::NoteError::NotFound(_) => Status::not_found("note not found"),
                other => Status::internal(other.to_string()),
            })?;
        Ok(Response::new(ProtoNote {
            id: note.id,
            body: note.body,
            created_at: note.created_at.to_rfc3339(),
            created_by: note.created_by,
        }))
    }

    async fn list(&self, req: Request<ListRequest>) -> Result<Response<ListResponse>, Status> {
        self.check_auth(&req).await?;
        let notes = self
            .store
            .list()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(ListResponse {
            notes: notes
                .into_iter()
                .map(|n| ProtoNote {
                    id: n.id,
                    body: n.body,
                    created_at: n.created_at.to_rfc3339(),
                    created_by: n.created_by,
                })
                .collect(),
        }))
    }
}
