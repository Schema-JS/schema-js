use crate::context::context::SjsContext;
use crate::error::SjsRtError;
use crate::runtime::SchemeJsRuntime;
use std::sync::Arc;

pub struct SjsPoolProvider {
    pub shared_context: Arc<SjsContext>,
}

impl r2d2::ManageConnection for SjsPoolProvider {
    type Connection = SchemeJsRuntime;
    type Error = SjsRtError;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let ctx = self.shared_context.clone();
        let current = tokio::runtime::Runtime::new().unwrap();

        let rt = current
            .block_on(async move { SchemeJsRuntime::new(ctx).await })
            .map_err(|e| SjsRtError::UnexpectedRuntimeCreation)?;

        Ok(rt)
    }

    fn is_valid(&self, _conn: &mut Self::Connection) -> Result<(), Self::Error> {
        Ok(())
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        conn.acquire_lock()
            .map_err(|_| Self::Error::BusyRuntime)
            .is_ok()
    }
}
