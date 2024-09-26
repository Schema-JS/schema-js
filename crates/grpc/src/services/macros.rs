#[macro_export]
macro_rules! define_sjs_grpc_service {
    ($service_name:ident) => {
        pub struct $service_name {
            db_manager: std::sync::Arc<schemajs_internal::manager::InternalManager>,
        }

        impl $service_name {
            pub fn new(
                db_manager: std::sync::Arc<schemajs_internal::manager::InternalManager>,
            ) -> Self {
                Self { db_manager }
            }
        }
    };
}
