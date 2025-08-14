use std::{
    ffi::OsString,
    ops::ControlFlow,
    path::{Path, PathBuf},
};

use mcc::{
    Text,
    codegen::asm,
    diagnostics::Diagnostics,
    lowering::tacky,
    target_lexicon::Triple,
    types::{Ast, SourceFile},
};

#[derive(Debug, Clone)]
pub struct Config {
    pub db: mcc::Database,
    pub target: Triple,
    pub cc: OsString,
    pub output: Option<PathBuf>,
    pub input: SourceFile,
}

/// Run the compiler.
///
/// This function is the entry point for the compiler. It will run the compiler
/// through the various stages of compilation, and call the appropriate
/// callbacks at each stage.
#[tracing::instrument(level = "info", skip_all)]
pub fn run<C: Callbacks>(cb: &mut C, cfg: Config) -> anyhow::Result<()> {
    let Config {
        db,
        target,
        cc,
        output,
        input,
    } = cfg;

    let temp = tempfile::tempdir()?;

    let preprocessed = mcc::preprocess(&db, cc.clone(), input)?;

    let preprocessed_path = temp.path().join("preprocessed.c");
    std::fs::write(&preprocessed_path, preprocessed)?;

    let ast = mcc::parse(&db, input);
    let diags: Vec<&Diagnostics> = mcc::parse::accumulated::<Diagnostics>(&db, input);
    if cb.after_parse(&db, input, ast, diags).is_break() {
        return Ok(());
    }

    let tacky = mcc::lowering::lower(&db, ast, input);
    let diags: Vec<&Diagnostics> =
        mcc::lowering::lower::accumulated::<Diagnostics>(&db, ast, input);
    if cb.after_lower(&db, tacky, diags).is_break() {
        return Ok(());
    }

    let program = mcc::codegen::generate_assembly(&db, tacky);
    let diags: Vec<&Diagnostics> =
        mcc::codegen::generate_assembly::accumulated::<Diagnostics>(&db, tacky);

    if cb.after_codegen(&db, program, diags).is_break() {
        return Ok(());
    }

    let assembly = mcc::render::render_program(&db, program, target.clone())?;
    let diags: Vec<&Diagnostics> =
        mcc::render::render_program::accumulated::<Diagnostics>(&db, program, target.clone());

    if cb
        .after_render_assembly(&db, assembly.clone(), diags)
        .is_break()
    {
        return Ok(());
    }

    let asm = temp.path().join("assembly.s");
    std::fs::write(&asm, assembly)?;

    let output_path = output
        .clone()
        .unwrap_or_else(|| Path::new(input.path(&db)).with_extension(""));

    mcc::assemble_and_link(&db, cc.clone(), asm, output_path.clone(), target.clone())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&output_path, perms)?;
    }

    if cb.after_compile(&db, output_path).is_break() {
        return Ok(());
    }

    Ok(())
}

/// Callbacks fired at various stages of compilation.
///
/// The callbacks are called in the following order:
///
/// 1. `after_parse`
/// 2. `after_lower`
/// 3. `after_codegen`
/// 4. `after_render_assembly`
/// 5. `after_compile`
///
/// If a callback returns `ControlFlow::Break`, the compiler will stop running
/// and return early.
pub trait Callbacks {
    /// Called after parsing the file.
    fn after_parse<'db>(
        &mut self,
        _db: &'db dyn mcc::Db,
        _source_file: mcc::types::SourceFile,
        _ast: Ast<'db>,
        _diags: Vec<&Diagnostics>,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    /// Called after lowering the [Abstract Syntax Tree][Ast] to [Three Address Code][tacky].
    fn after_lower<'db>(
        &mut self,
        _db: &'db dyn mcc::Db,
        _tacky: tacky::Program<'db>,
        _diags: Vec<&Diagnostics>,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    /// Called after generating [assembly instructions][asm] from [Three Address Code][tacky].
    fn after_codegen<'db>(
        &mut self,
        _db: &'db dyn mcc::Db,
        _asm: asm::Program<'db>,
        _diags: Vec<&Diagnostics>,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    /// Called after rendering the [assembly instructions][asm] to a string.
    fn after_render_assembly(
        &mut self,
        _db: &dyn mcc::Db,
        _asm: Text,
        _diags: Vec<&Diagnostics>,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn after_compile(&mut self, _db: &dyn mcc::Db, _binary: PathBuf) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
}
