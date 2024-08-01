pub mod emitter;
pub mod eszip;
pub mod eszip_fs;
pub mod eszip_util;

use ::eszip::deno_graph;
use ::eszip::deno_graph::source::{ResolutionMode, ResolveError, Resolver};
use deno_core::error::AnyError;
use deno_core::ModuleSpecifier;
use import_map::ImportMap;
use std::sync::Arc;

/// Resolver for specifiers that could be mapped via an
/// import map or package.json.
#[derive(Debug)]
pub struct MappedSpecifierResolver {
    maybe_import_map: Option<Arc<ImportMap>>,
}

/// Result of checking if a specifier is mapped via
/// an import map or package.json.
pub enum MappedResolution {
    None,
    ImportMap(ModuleSpecifier),
}

impl MappedResolution {
    pub fn into_specifier(self) -> Option<ModuleSpecifier> {
        match self {
            MappedResolution::None => None,
            MappedResolution::ImportMap(specifier) => Some(specifier),
        }
    }
}

impl MappedSpecifierResolver {
    pub fn new(maybe_import_map: Option<Arc<ImportMap>>) -> Self {
        Self { maybe_import_map }
    }

    pub fn resolve(
        &self,
        specifier: &str,
        referrer: &ModuleSpecifier,
    ) -> Result<MappedResolution, AnyError> {
        // attempt to resolve with the import map first
        let maybe_import_map_err = match self
            .maybe_import_map
            .as_ref()
            .map(|import_map| import_map.resolve(specifier, referrer))
        {
            Some(Ok(value)) => return Ok(MappedResolution::ImportMap(value)),
            Some(Err(err)) => Some(err),
            None => None,
        };

        // otherwise, surface the import map error or try resolving when has no import map
        if let Some(err) = maybe_import_map_err {
            Err(err.into())
        } else {
            Ok(MappedResolution::None)
        }
    }
}

/// A resolver that takes care of resolution, taking into account loaded
/// import map, JSX settings.
#[derive(Debug)]
pub struct CliGraphResolver {
    mapped_specifier_resolver: MappedSpecifierResolver,
}

#[derive(Default)]
pub struct CliGraphResolverOptions {
    pub maybe_import_map: Option<Arc<ImportMap>>,
}

impl CliGraphResolver {
    pub fn new(options: CliGraphResolverOptions) -> Self {
        Self {
            mapped_specifier_resolver: MappedSpecifierResolver {
                maybe_import_map: options.maybe_import_map,
            },
        }
    }
    pub fn as_graph_resolver(&self) -> &dyn Resolver {
        self
    }
}

impl Resolver for CliGraphResolver {
    fn resolve(
        &self,
        specifier: &str,
        referrer_range: &deno_graph::Range,
        _mode: ResolutionMode,
    ) -> Result<ModuleSpecifier, ResolveError> {
        let referrer = &referrer_range.specifier;
        let result = self
            .mapped_specifier_resolver
            .resolve(specifier, referrer)
            .map_err(|err| err.into())
            .and_then(|resolution| match resolution {
                MappedResolution::ImportMap(specifier) => Ok(specifier),
                MappedResolution::None => {
                    deno_graph::resolve_import(specifier, &referrer_range.specifier)
                        .map_err(|err| err.into())
                }
            });

        result
    }
}
