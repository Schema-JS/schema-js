use deno_core::{FastString, ModuleLoader};
use std::rc::Rc;

pub struct RuntimeProviders {
    pub module_loader: Rc<dyn ModuleLoader>,
    pub module_code: Option<FastString>,
}
