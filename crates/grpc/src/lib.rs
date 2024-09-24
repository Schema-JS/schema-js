mod interceptors;
mod server;
mod services;

pub type GrpcResponse<T> = Result<tonic::Response<T>, tonic::Status>;
