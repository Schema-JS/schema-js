use deno_ast::{MediaType, ParseParams, SourceMapOption, SourceTextInfo};
use deno_core::error::AnyError;
use deno_core::{ModuleCodeString, ModuleName, SourceMapData};
use std::path::Path;
use std::sync::Arc;

/// Obtained from: https://github.com/denoland/deno/blob/cd59fc53a528603112addfe8b10fe4e30d04e7f0/runtime/shared.rs#L67
pub fn maybe_transpile_source(
    name: ModuleName,
    source: ModuleCodeString,
) -> Result<(ModuleCodeString, Option<SourceMapData>), AnyError> {
    // Always transpile `node:` built-in modules, since they might be TypeScript.
    let media_type = if name.starts_with("node:") {
        MediaType::TypeScript
    } else {
        MediaType::from_path(Path::new(&name))
    };

    match media_type {
        MediaType::TypeScript => {}
        MediaType::JavaScript => return Ok((source, None)),
        MediaType::Mjs => return Ok((source, None)),
        _ => panic!(
            "Unsupported media type for snapshotting {media_type:?} for file {}",
            name
        ),
    }

    let parsed = deno_ast::parse_module(ParseParams {
        specifier: deno_core::url::Url::parse(&name).unwrap(),
        text: source.into(),
        media_type,
        capture_tokens: false,
        scope_analysis: false,
        maybe_syntax: None,
    })?;

    let transpiled_source = parsed
        .transpile(
            &deno_ast::TranspileOptions {
                imports_not_used_as_values: deno_ast::ImportsNotUsedAsValues::Remove,
                ..Default::default()
            },
            &deno_ast::EmitOptions {
                source_map: if cfg!(debug_assertions) {
                    SourceMapOption::Separate
                } else {
                    SourceMapOption::None
                },
                ..Default::default()
            },
        )?
        .into_source();

    let maybe_source_map: Option<SourceMapData> = transpiled_source.source_map.map(|sm| sm.into());
    let source_text = String::from_utf8(transpiled_source.source)?;

    Ok((source_text.into(), maybe_source_map))
}
