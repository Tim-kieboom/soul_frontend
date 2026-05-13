use crate::paths::Paths;
use anyhow::Result;
use ast::AbtractSyntaxTree;
use run_hir::HirResponse;
use run_mir::MirResponse;
use soul_tokenizer::to_token_stream;
use soul_utils::{ModuleId, sementic_level::ModuleStore};
use std::path::Path;

pub(crate) mod displayer_ast;
pub(crate) mod displayer_hir;
pub(crate) mod displayer_mir;
pub(crate) mod displayer_soul_error;
pub(crate) mod displayer_tokenizer;

pub(crate) fn display_tokenizer(
    paths: &Paths,
    manifest: &Path,
    module: ModuleId,
    source_file: &str,
) -> Result<()> {
    let token_stream = to_token_stream(source_file, module);
    let tokens = displayer_tokenizer::display_tokens(paths, source_file, token_stream)?;
    Paths::write_to_output(&tokens, manifest, Path::new("tokenizer\\tokens.soulc"))
}

pub(crate) fn display_ast(
    manifest: &Path,
    module_store: &ModuleStore,
    ast_context: &AbtractSyntaxTree,
) -> Result<()> {
    let root = module_store.get_root_id();
    Paths::write_to_output(
        &displayer_ast::display_ast(root, module_store, ast_context),
        manifest,
        Path::new("ast\\tree.soulc"),
    )?;
    Paths::write_to_output(
        &displayer_ast::display_ast_name_resolved(root, module_store, ast_context),
        manifest,
        Path::new("ast\\NameResolved.soulc"),
    )
}

pub(crate) fn display_hir(
    manifest: &Path,
    hir: &HirResponse,
    ast_context: &AbtractSyntaxTree,
) -> Result<()> {
    Paths::write_to_output(
        &displayer_hir::display_hir(ast_context, &hir.hir),
        manifest,
        Path::new("hir\\tree.soulc"),
    )?;
    Paths::write_to_output(
        &displayer_hir::display_thir(ast_context, &hir.hir, &hir.typed),
        manifest,
        Path::new("thir\\tree.soulc"),
    )?;
    Paths::write_to_output(
        &displayer_hir::display_created_types(&hir.hir, &hir.typed),
        manifest,
        Path::new("thir\\types.soulc"),
    )?;
    Ok(())
}

pub(crate) fn display_mir(
    manifest: &Path,
    mir: &MirResponse,
    hir: &HirResponse,
    ast_context: &AbtractSyntaxTree,
) -> Result<()> {
    Paths::write_to_output(
        &displayer_mir::display_mir(&mir.tree, hir, &ast_context.modules),
        manifest,
        Path::new("mir\\tree.soulc"),
    )
}
