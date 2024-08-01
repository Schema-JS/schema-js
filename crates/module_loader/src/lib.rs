pub mod runtime_providers;
pub mod standalone;
pub mod util;

use crate::runtime_providers::RuntimeProviders;
use crate::standalone::{EmbeddedModuleLoader, SharedModuleLoaderState};
use anyhow::Context;
use deno_core::error::AnyError;
use deno_core::url::Url;
use deno_core::{FastString, ModuleSpecifier};
use import_map::{parse_from_json, ImportMap};
use schemajs_core::util::deno_dir::DenoDirProvider;
use schemajs_core::util::http_util::HttpClient;
use schemajs_graph::eszip::{payload_to_eszip, EszipPayloadKind, SOURCE_CODE_ESZIP_KEY};
use schemajs_graph::MappedSpecifierResolver;
use std::rc::Rc;
use std::sync::Arc;

pub struct StandaloneModuleLoaderFactory {
    shared: Arc<SharedModuleLoaderState>,
}

pub async fn create_module_loader_for_eszip(
    mut eszip: eszip::EszipV2,
    maybe_import_map: Option<ImportMap>,
    include_source_map: bool,
) -> Result<RuntimeProviders, AnyError> {
    let code_fs = if let Some(module) = eszip.get_module(SOURCE_CODE_ESZIP_KEY) {
        if let Some(code) = module.take_source().await {
            Some(FastString::from(String::from_utf8(code.to_vec())?))
        } else {
            None
        }
    } else {
        None
    };

    let maybe_import_map = maybe_import_map
        .map(|import_map| Some(Arc::new(import_map)))
        .unwrap_or_else(|| None);

    let module_loader_factory = StandaloneModuleLoaderFactory {
        shared: Arc::new(SharedModuleLoaderState {
            eszip,
            mapped_specifier_resolver: MappedSpecifierResolver::new(maybe_import_map),
        }),
    };

    Ok(RuntimeProviders {
        module_loader: Rc::new(EmbeddedModuleLoader {
            shared: module_loader_factory.shared.clone(),
            include_source_map,
        }),
        module_code: code_fs,
    })
}

pub async fn create_module_loader_for_standalone_from_eszip_kind(
    eszip_payload_kind: EszipPayloadKind,
    maybe_import_map_arc: Option<Arc<ImportMap>>,
    maybe_import_map_path: Option<String>,
    include_source_map: bool,
) -> Result<RuntimeProviders, AnyError> {
    let eszip = payload_to_eszip(eszip_payload_kind).await;

    let mut maybe_import_map: Option<ImportMap> = None;

    if let Some(import_map) = maybe_import_map_arc {
        let clone_import_map = (*import_map).clone();
        maybe_import_map = Some(clone_import_map);
    } else if let Some(import_map_path) = maybe_import_map_path {
        let import_map_url = Url::parse(import_map_path.as_str())?;
        if let Some(import_map_module) = eszip.get_import_map(import_map_url.as_str()) {
            if let Some(source) = import_map_module.source().await {
                let source = std::str::from_utf8(&source)?.to_string();
                let result = parse_from_json(import_map_url.clone(), &source)?;
                maybe_import_map = Some(result.import_map);
            }
        }
    }

    create_module_loader_for_eszip(eszip, maybe_import_map, include_source_map).await
}
