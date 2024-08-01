// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use base64::Engine;
use deno_core::error::generic_error;
use deno_core::error::type_error;
use deno_core::error::AnyError;
use deno_core::futures::FutureExt;
use deno_core::ModuleType;
use deno_core::ResolutionKind;
use deno_core::{ModuleLoader, ModuleSourceCode};
use deno_core::{ModuleSpecifier, RequestedModuleType};
use eszip::deno_graph;
use std::sync::Arc;

use crate::util::arc_u8_to_arc_str;
use schemajs_graph::MappedSpecifierResolver;

pub struct SharedModuleLoaderState {
    pub(crate) eszip: eszip::EszipV2,
    pub(crate) mapped_specifier_resolver: MappedSpecifierResolver,
}

#[derive(Clone)]
pub struct EmbeddedModuleLoader {
    pub(crate) shared: Arc<SharedModuleLoaderState>,
    pub(crate) include_source_map: bool,
}

impl ModuleLoader for EmbeddedModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        kind: ResolutionKind,
    ) -> Result<ModuleSpecifier, AnyError> {
        let referrer = if referrer == "." {
            if kind != ResolutionKind::MainModule {
                return Err(generic_error(format!(
                    "Expected to resolve main module, got {:?} instead.",
                    kind
                )));
            }
            let current_dir = std::env::current_dir().unwrap();
            deno_core::resolve_path(".", &current_dir)?
        } else {
            ModuleSpecifier::parse(referrer)
                .map_err(|err| type_error(format!("Referrer uses invalid specifier: {}", err)))?
        };

        let maybe_mapped = self
            .shared
            .mapped_specifier_resolver
            .resolve(specifier, &referrer)?
            .into_specifier();

        let specifier = match maybe_mapped {
            Some(resolved) => resolved,
            None => deno_core::resolve_import(specifier, referrer.as_str())?,
        };

        Ok(specifier)
    }

    fn load(
        &self,
        original_specifier: &ModuleSpecifier,
        maybe_referrer: Option<&ModuleSpecifier>,
        _is_dynamic: bool,
        _requested_module_type: RequestedModuleType,
    ) -> deno_core::ModuleLoadResponse {
        let include_source_map = self.include_source_map;

        if original_specifier.scheme() == "data" {
            let data_url_text = match deno_graph::source::RawDataUrl::parse(original_specifier)
                .and_then(|url| url.decode().map_err(|err| err.into()))
            {
                Ok(response) => response,
                Err(err) => {
                    return deno_core::ModuleLoadResponse::Sync(Err(type_error(format!(
                        "{:#}",
                        err
                    ))));
                }
            };
            return deno_core::ModuleLoadResponse::Sync(Ok(deno_core::ModuleSource::new(
                deno_core::ModuleType::JavaScript,
                ModuleSourceCode::String(data_url_text.into()),
                original_specifier,
                None,
            )));
        }

        let Some(module) = self.shared.eszip.get_module(original_specifier.as_str()) else {
            return deno_core::ModuleLoadResponse::Sync(Err(type_error(format!(
                "Module not found: {}",
                original_specifier
            ))));
        };
        let original_specifier = original_specifier.clone();
        let found_specifier =
            ModuleSpecifier::parse(&module.specifier).expect("invalid url in eszip");

        deno_core::ModuleLoadResponse::Async(
            async move {
                let code = module
                    .source()
                    .await
                    .ok_or_else(|| type_error(format!("Module not found: {}", original_specifier)))
                    .and_then(|it| {
                        arc_u8_to_arc_str(it).map_err(|_| type_error("Module source is not utf-8"))
                    })?;

                let source_map = module.source_map().await;
                let maybe_code_with_source_map = 'scope: {
                    if !include_source_map {
                        break 'scope code;
                    }

                    let Some(source_map) = source_map else {
                        break 'scope code;
                    };

                    let mut src = code.to_string();

                    if src.ends_with('\n') {
                        src.push('\n');
                    }

                    src.push_str("//# sourceMappingURL=data:application/json;base64,");
                    base64::prelude::BASE64_STANDARD.encode_string(source_map, &mut src);

                    Arc::from(src)
                };

                Ok(deno_core::ModuleSource::new_with_redirect(
                    match module.kind {
                        eszip::ModuleKind::JavaScript => ModuleType::JavaScript,
                        eszip::ModuleKind::Json => ModuleType::Json,
                        eszip::ModuleKind::Jsonc => {
                            return Err(type_error("jsonc modules not supported"))
                        }
                        eszip::ModuleKind::OpaqueData => {
                            unreachable!();
                        }
                    },
                    ModuleSourceCode::String(maybe_code_with_source_map.into()),
                    &original_specifier,
                    &found_specifier,
                    None,
                ))
            }
            .boxed_local(),
        )
    }
}
