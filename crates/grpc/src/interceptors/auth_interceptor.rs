use schemajs_internal::manager::InternalManager;
use std::sync::Arc;
use tonic::body::BoxBody;
use tonic::{async_trait, Request, Status};
use tonic_middleware::RequestInterceptor;

#[derive(Clone)]
pub struct AuthInterceptor {
    pub(crate) engine: Arc<InternalManager>,
}

#[async_trait]
impl RequestInterceptor for AuthInterceptor {
    async fn intercept(
        &self,
        mut req: tonic::codegen::http::Request<BoxBody>,
    ) -> Result<tonic::codegen::http::Request<BoxBody>, Status> {
        match req.headers().get("x-sjs-auth") {
            None => Err(Status::unauthenticated("Unknown Authentication")),
            Some(val) => {
                let ctx = self
                    .engine
                    .auth_manager()
                    .check_token(val.to_str().unwrap());
                if let Ok(user_ctx) = ctx {
                    req.extensions_mut().insert(user_ctx);
                    Ok(req)
                } else {
                    Err(Status::unauthenticated("Unknown Authentication"))
                }
            }
        }
    }
}
