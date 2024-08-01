use crate::eszip::DecoratorType;
use crate::{CliGraphResolver, CliGraphResolverOptions};
use deno_ast::{EmitOptions, SourceMapOption, TranspileOptions};
use deno_core::error::AnyError;
use eszip::deno_graph::source::Loader;
use import_map::ImportMap;
use schemajs_core::cache::parsed_source_cache::ParsedSourceCache;
use schemajs_core::cache::{CacheSetting, GlobalHttpCache, RealDenoCacheEnv};
use schemajs_core::emit::Emitter;
use schemajs_core::loaders::file_fetcher::{FileCache, FileFetcher};
use schemajs_core::util::caches::Caches;
use schemajs_core::util::deno_dir::{DenoDir, DenoDirProvider};
use schemajs_core::util::emit::EmitCache;
use schemajs_core::util::fetch_cacher::FetchCacher;
use schemajs_core::util::http_util::HttpClient;
use schemajs_core::util::module_info::ModuleInfoCache;
use schemajs_core::util::HttpCache;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

struct Deferred<T>(once_cell::unsync::OnceCell<T>);

impl<T> Default for Deferred<T> {
    fn default() -> Self {
        Self(once_cell::unsync::OnceCell::default())
    }
}

impl<T> Deferred<T> {
    #[allow(dead_code)]
    pub fn get_or_try_init(
        &self,
        create: impl FnOnce() -> Result<T, AnyError>,
    ) -> Result<&T, AnyError> {
        self.0.get_or_try_init(create)
    }

    pub fn get_or_init(&self, create: impl FnOnce() -> T) -> &T {
        self.0.get_or_init(create)
    }

    #[allow(dead_code)]
    pub async fn get_or_try_init_async(
        &self,
        create: impl Future<Output = Result<T, AnyError>>,
    ) -> Result<&T, AnyError> {
        if self.0.get().is_none() {
            // todo(dsherret): it would be more ideal if this enforced a
            // single executor and then we could make some initialization
            // concurrent
            let val = create.await?;
            _ = self.0.set(val);
        }
        Ok(self.0.get().unwrap())
    }
}

pub struct EmitterFactory {
    deno_dir: DenoDir,
    maybe_decorator: Option<DecoratorType>,
    resolver: Deferred<Arc<CliGraphResolver>>,
    file_fetcher_cache_strategy: Option<CacheSetting>,
    file_fetcher_allow_remote: bool,
    pub maybe_import_map: Option<Arc<ImportMap>>,
    file_cache: Deferred<Arc<FileCache>>,
    module_info_cache: Deferred<Arc<ModuleInfoCache>>,
}

impl Default for EmitterFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl EmitterFactory {
    pub fn new() -> Self {
        let deno_dir = DenoDir::new(None).unwrap();

        Self {
            module_info_cache: Default::default(),
            deno_dir,
            maybe_decorator: None,
            resolver: Default::default(),
            file_fetcher_cache_strategy: None,
            file_fetcher_allow_remote: true,
            maybe_import_map: None,
            file_cache: Default::default(),
        }
    }

    pub fn set_file_fetcher_cache_strategy(&mut self, strategy: CacheSetting) {
        self.file_fetcher_cache_strategy = Some(strategy);
    }

    pub fn set_file_fetcher_allow_remote(&mut self, allow_remote: bool) {
        self.file_fetcher_allow_remote = allow_remote;
    }

    pub fn set_import_map(&mut self, import_map: Option<ImportMap>) {
        self.maybe_import_map = import_map
            .map(|import_map| Some(Arc::new(import_map)))
            .unwrap_or_else(|| None);
    }

    pub fn set_decorator_type(&mut self, decorator_type: Option<DecoratorType>) {
        self.maybe_decorator = decorator_type;
    }
    pub fn deno_dir_provider(&self) -> Arc<DenoDirProvider> {
        Arc::new(DenoDirProvider::new(None))
    }

    pub fn caches(&self) -> Result<Arc<Caches>, AnyError> {
        let caches = Arc::new(Caches::new(self.deno_dir_provider()));
        let _ = caches.dep_analysis_db();
        Ok(caches)
    }

    pub fn module_info_cache(&self) -> Result<&Arc<ModuleInfoCache>, AnyError> {
        self.module_info_cache.get_or_try_init(|| {
            Ok(Arc::new(ModuleInfoCache::new(
                self.caches()?.dep_analysis_db(),
            )))
        })
    }

    pub fn emit_cache(&self, transpile_options: TranspileOptions) -> Result<EmitCache, AnyError> {
        Ok(EmitCache::new(
            self.deno_dir.gen_cache.clone(),
            transpile_options,
        ))
    }

    pub fn parsed_source_cache(&self) -> Result<Arc<ParsedSourceCache>, AnyError> {
        let source_cache = Arc::new(ParsedSourceCache::default());
        Ok(source_cache)
    }

    pub fn emit_options(&self) -> EmitOptions {
        EmitOptions {
            inline_sources: true,
            source_map: SourceMapOption::Inline,
            ..Default::default()
        }
    }

    pub fn transpile_options(&self) -> TranspileOptions {
        TranspileOptions {
            use_decorators_proposal: self
                .maybe_decorator
                .map(DecoratorType::is_use_decorators_proposal)
                .unwrap_or_default(),

            use_ts_decorators: self
                .maybe_decorator
                .map(DecoratorType::is_use_ts_decorators)
                .unwrap_or_default(),

            emit_metadata: self
                .maybe_decorator
                .map(DecoratorType::is_emit_metadata)
                .unwrap_or_default(),
            ..Default::default()
        }
    }

    pub fn emitter(&self) -> Result<Arc<Emitter>, AnyError> {
        let transpile_options = self.transpile_options();
        let emitter = Arc::new(Emitter::new(
            self.emit_cache(transpile_options.clone())?,
            self.parsed_source_cache()?,
            self.emit_options(),
            transpile_options,
        ));

        Ok(emitter)
    }

    pub fn global_http_cache(&self) -> GlobalHttpCache {
        GlobalHttpCache::new(self.deno_dir.deps_folder_path(), RealDenoCacheEnv)
    }

    pub fn http_client(&self) -> Arc<HttpClient> {
        let http_client = HttpClient::new(None);

        Arc::new(http_client)
    }

    pub fn real_fs(&self) -> Arc<dyn deno_fs::FileSystem> {
        Arc::new(deno_fs::RealFs)
    }

    pub fn file_cache(&self) -> &Arc<FileCache> {
        self.file_cache.get_or_init(Default::default)
    }

    pub async fn cli_graph_resolver(&self) -> &Arc<CliGraphResolver> {
        self.resolver
            .get_or_try_init_async(async {
                Ok(Arc::new(CliGraphResolver::new(
                    self.cli_graph_resolver_options(),
                )))
            })
            .await
            .unwrap()
    }

    pub fn cli_graph_resolver_options(&self) -> CliGraphResolverOptions {
        CliGraphResolverOptions {
            maybe_import_map: self.maybe_import_map.clone(),
        }
    }

    pub fn file_fetcher(&self) -> FileFetcher {
        let global_cache_struct =
            GlobalHttpCache::new(self.deno_dir.deps_folder_path(), RealDenoCacheEnv);
        let global_cache: Arc<dyn HttpCache> = Arc::new(global_cache_struct);
        let http_client = self.http_client();
        let blob_store = Arc::new(deno_web::BlobStore::default());

        FileFetcher::new(
            global_cache.clone(),
            self.file_fetcher_cache_strategy
                .clone()
                .unwrap_or(CacheSetting::ReloadAll),
            self.file_fetcher_allow_remote,
            http_client,
            blob_store,
            self.file_cache().clone(),
        )
    }

    pub fn file_fetcher_loader(&self) -> Box<dyn Loader> {
        let global_cache_struct =
            GlobalHttpCache::new(self.deno_dir.deps_folder_path(), RealDenoCacheEnv);

        let parsed_source = self.parsed_source_cache().unwrap();

        Box::new(FetchCacher::new(
            self.module_info_cache().unwrap().clone(),
            self.emit_cache(self.transpile_options()).unwrap(),
            Arc::new(self.file_fetcher()),
            HashMap::new(),
            Arc::new(global_cache_struct),
            parsed_source,
            None, // TODO: NPM
        ))
    }
}
