use schemajs_engine::engine::SchemeJsEngine;
use std::sync::Arc;
use tonic::transport::Server;

pub struct GrpcServer {
    db: Arc<SchemeJsEngine>,
}

impl GrpcServer {
    pub fn new(db: Arc<SchemeJsEngine>) -> Self {
        Self { db }
    }

    pub fn start(&self) {
        /* Server::builder().layer()*/
    }
}
