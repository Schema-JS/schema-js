#[macro_export]
macro_rules! define_sjs_grpc_service {
    // Pattern to accept service name and additional methods
    ($service_name:ident, { $($methods:item)* }) => {
        pub struct $service_name {
            db_manager: std::sync::Arc<schemajs_internal::manager::InternalManager>,
        }

        impl $service_name {
            pub fn new(
                db_manager: std::sync::Arc<schemajs_internal::manager::InternalManager>,
            ) -> Self {
                Self { db_manager }
            }

            // Insert the custom methods provided by the user
            $($methods)*
        }
    };
    // Pattern to accept only service name without additional methods
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
