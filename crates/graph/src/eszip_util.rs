use crate::emitter::EmitterFactory;
use crate::eszip_fs::DenoGraphFsAdapter;
use crate::CliGraphResolver;
use deno_ast::MediaType;
use deno_core::error::{custom_error, AnyError};
use deno_core::{FastString, ModuleSpecifier};
use deno_graph::ModuleError;
use deno_graph::ResolutionError;
use eszip::deno_graph::source::{DefaultJsrUrlProvider, JsrUrlProvider, Loader};
use eszip::deno_graph::{GraphKind, ModuleGraph, ModuleGraphError};
use eszip::{deno_graph, EszipV2, FromGraphOptions};
use schemajs_core::cache::parsed_source_cache::ParsedSourceCache;
use schemajs_core::loaders::file_fetcher::File;
use schemajs_core::util::errors_rt::get_error_class_name;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Copy)]
pub struct GraphValidOptions {
    pub check_js: bool,
    pub follow_type_only: bool,
    pub is_vendoring: bool,
}

/// Check if `roots` and their deps are available. Returns `Ok(())` if
/// so. Returns `Err(_)` if there is a known module graph or resolution
/// error statically reachable from `roots` and not a dynamic import.
pub fn graph_valid_with_cli_options(
    graph: &ModuleGraph,
    roots: &[ModuleSpecifier],
) -> Result<(), AnyError> {
    graph_valid(
        graph,
        roots,
        GraphValidOptions {
            is_vendoring: false,
            follow_type_only: true,
            check_js: false,
        },
    )
}

pub struct ModuleGraphBuilder {
    type_check: bool,
    emitter_factory: Arc<EmitterFactory>,
}

impl ModuleGraphBuilder {
    pub fn new(emitter_factory: Arc<EmitterFactory>, type_check: bool) -> Self {
        Self {
            type_check,
            emitter_factory,
        }
    }

    pub async fn resolver(&self) -> Arc<CliGraphResolver> {
        self.emitter_factory.cli_graph_resolver().await.clone()
    }

    pub fn parsed_source_cache(&self) -> Arc<ParsedSourceCache> {
        self.emitter_factory.parsed_source_cache().unwrap().clone()
    }

    pub async fn create_graph_with_loader(
        &self,
        graph_kind: GraphKind,
        roots: Vec<ModuleSpecifier>,
        loader: &mut dyn Loader,
    ) -> Result<deno_graph::ModuleGraph, AnyError> {
        let cli_resolver = self.resolver().await;
        let graph_resolver = cli_resolver.as_graph_resolver();
        let psc = self.parsed_source_cache();
        let parser = psc.as_capturing_parser();
        let analyzer = self
            .emitter_factory
            .module_info_cache()
            .unwrap()
            .as_module_analyzer(&parser);

        let mut graph = ModuleGraph::new(graph_kind);
        let fs = Arc::new(deno_fs::RealFs);
        let fs = DenoGraphFsAdapter(fs.as_ref());
        let jsr: &dyn JsrUrlProvider = Default::default();

        self.build_graph_with_npm_resolution(
            &mut graph,
            roots,
            loader,
            deno_graph::BuildOptions {
                is_dynamic: false,
                imports: vec![],
                executor: Default::default(),
                locker: None,
                file_system: &fs,
                jsr_url_provider: jsr,
                resolver: Some(graph_resolver),
                npm_resolver: None,
                module_analyzer: &analyzer,
                reporter: None,
                workspace_members: &[],
                passthrough_jsr_specifiers: false,
            },
        )
        .await?;

        Ok(graph)
    }

    pub async fn build_graph_with_npm_resolution<'a>(
        &self,
        graph: &mut ModuleGraph,
        roots: Vec<ModuleSpecifier>,
        loader: &'a mut dyn deno_graph::source::Loader,
        options: deno_graph::BuildOptions<'a>,
    ) -> Result<(), AnyError> {
        graph.build(roots, loader, options).await;
        Ok(())
    }

    #[allow(clippy::borrow_deref_ref)]
    pub async fn create_graph_and_maybe_check(
        &self,
        roots: Vec<ModuleSpecifier>,
    ) -> Result<deno_graph::ModuleGraph, AnyError> {
        //
        let mut cache = self.emitter_factory.file_fetcher_loader();
        let cli_resolver = self.resolver().await.clone();
        let graph_resolver = cli_resolver.as_graph_resolver();
        let psc = self.parsed_source_cache();
        let parser = psc.as_capturing_parser();
        let analyzer = self
            .emitter_factory
            .module_info_cache()
            .unwrap()
            .as_module_analyzer(&parser);
        let graph_kind = deno_graph::GraphKind::CodeOnly;
        let mut graph = ModuleGraph::new(graph_kind);
        let fs = Arc::new(deno_fs::RealFs);
        let fs = DenoGraphFsAdapter(fs.as_ref());
        let jsr: &dyn JsrUrlProvider = Default::default();

        self.build_graph_with_npm_resolution(
            &mut graph,
            roots,
            cache.as_mut(),
            deno_graph::BuildOptions {
                is_dynamic: false,
                imports: vec![],
                executor: Default::default(),
                locker: None,
                file_system: &fs,
                jsr_url_provider: jsr,
                resolver: Some(&*graph_resolver),
                npm_resolver: None,
                module_analyzer: &analyzer,
                reporter: None,
                workspace_members: &[],
                passthrough_jsr_specifiers: false,
            },
        )
        .await?;

        Ok(graph)
    }
}

/// Check if `roots` and their deps are available. Returns `Ok(())` if
/// so. Returns `Err(_)` if there is a known module graph or resolution
/// error statically reachable from `roots`.
///
/// It is preferable to use this over using deno_graph's API directly
/// because it will have enhanced error message information specifically
/// for the CLI.
pub fn graph_valid(
    graph: &ModuleGraph,
    roots: &[ModuleSpecifier],
    options: GraphValidOptions,
) -> Result<(), AnyError> {
    let mut errors = graph
        .walk(
            roots.into_iter(),
            deno_graph::WalkOptions {
                check_js: options.check_js,
                follow_type_only: options.follow_type_only,
                follow_dynamic: options.is_vendoring,
                prefer_fast_check_graph: false,
            },
        )
        .errors()
        .flat_map(|error| {
            let _is_root = match &error {
                ModuleGraphError::ResolutionError(_) => false,
                ModuleGraphError::ModuleError(error) => roots.contains(error.specifier()),
                _ => false,
            };
            let message = if let ModuleGraphError::ResolutionError(_err) = &error {
                format!("{error}")
            } else {
                format!("{error}")
            };

            if options.is_vendoring {
                // warn about failing dynamic imports when vendoring, but don't fail completely
                if matches!(
                    error,
                    ModuleGraphError::ModuleError(ModuleError::MissingDynamic(_, _))
                ) {
                    return None;
                }

                // ignore invalid downgrades and invalid local imports when vendoring
                if let ModuleGraphError::ResolutionError(err) = &error {
                    if matches!(
                        err,
                        ResolutionError::InvalidDowngrade { .. }
                            | ResolutionError::InvalidLocalImport { .. }
                    ) {
                        return None;
                    }
                }
            }

            Some(custom_error(
                get_error_class_name(&error.into()).unwrap(),
                message,
            ))
        });
    if let Some(error) = errors.next() {
        Err(error)
    } else {
        Ok(())
    }
}

#[allow(clippy::arc_with_non_send_sync)]
pub async fn create_eszip_from_graph_raw(
    graph: ModuleGraph,
    emitter_factory: Option<Arc<EmitterFactory>>,
) -> Result<EszipV2, AnyError> {
    let emitter = emitter_factory.unwrap_or_else(|| Arc::new(EmitterFactory::new()));
    let parser_arc = emitter.clone().parsed_source_cache().unwrap();
    let parser = parser_arc.as_capturing_parser();

    eszip::EszipV2::from_graph(FromGraphOptions {
        graph,
        parser,
        transpile_options: emitter.transpile_options(),
        emit_options: emitter.emit_options(),
        relative_file_base: None,
    })
}

pub async fn create_graph(
    file: PathBuf,
    emitter_factory: Arc<EmitterFactory>,
    maybe_code: &Option<FastString>,
) -> ModuleGraph {
    let module_specifier = if let Some(code) = maybe_code {
        let specifier = ModuleSpecifier::parse("file:///src/index.ts").unwrap();

        emitter_factory.file_cache().insert(
            specifier.clone(),
            File {
                maybe_types: None,
                media_type: MediaType::TypeScript,
                source: code.as_str().into(),
                specifier: specifier.clone(),
                maybe_headers: None,
            },
        );

        specifier
    } else {
        let binding = std::fs::canonicalize(&file).unwrap();
        let specifier = binding.to_str().unwrap();
        let format_specifier = format!("file:///{}", specifier);

        ModuleSpecifier::parse(&format_specifier).unwrap()
    };

    let builder = ModuleGraphBuilder::new(emitter_factory, false);

    let create_module_graph_task = builder.create_graph_and_maybe_check(vec![module_specifier]);
    create_module_graph_task.await.unwrap()
}
