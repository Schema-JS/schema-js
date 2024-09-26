mod interceptors;
pub mod server;
mod services;

pub type GrpcResponse<T> = Result<tonic::Response<T>, tonic::Status>;
