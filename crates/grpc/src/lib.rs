pub mod interceptors;
pub mod server;
mod services;
mod utils;

pub type GrpcResponse<T> = Result<tonic::Response<T>, tonic::Status>;
