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

#[derive(Debug)]
pub enum Outcome<Ret> {
    /// The compilation succeeded.
    Ok,
    /// The compilation failed.
    Err(anyhow::Error),
    /// The compilation returned early.
    EarlyReturn(Ret),
}

impl<Ret> Outcome<Ret> {
    pub fn to_result_with(
        self,
        f: impl FnOnce(Ret) -> Result<(), anyhow::Error>,
    ) -> Result<(), anyhow::Error> {
        match self {
            Self::Ok => Ok(()),
            Self::Err(e) => Err(e),
            Self::EarlyReturn(ret) => f(ret),
        }
    }

    pub fn to_result(self) -> Result<(), anyhow::Error> {
        self.to_result_with(|_| Err(anyhow::anyhow!("returned early")))
    }
}

impl<Ret> From<anyhow::Error> for Outcome<Ret> {
    fn from(err: anyhow::Error) -> Self {
        Self::Err(err)
    }
}

/// Run the compiler.
///
/// This function is the entry point for the compiler. It will run the compiler
/// through the various stages of compilation, and call the appropriate
/// callbacks at each stage.
#[tracing::instrument(level = "info", skip_all)]
pub fn run<C: Callbacks>(cb: &mut C, cfg: Config) -> Outcome<C::Output> {
    let Config {
        db,
        target,
        cc,
        output,
        input,
    } = cfg;

    let temp =
        match tempfile::tempdir().map_err(|e| anyhow::anyhow!("failed to create temp dir: {e}")) {
            Ok(temp) => temp,
            Err(e) => return Outcome::Err(e),
        };

    let preprocessed = match mcc::preprocess(&db, cc.clone(), input)
        .map_err(|e| anyhow::anyhow!("failed to preprocess: {e}"))
    {
        Ok(preprocessed) => preprocessed,
        Err(e) => return Outcome::Err(e),
    };

    let preprocessed_path = temp.path().join("preprocessed.c");

    if let Err(e) = std::fs::write(&preprocessed_path, preprocessed) {
        return Outcome::Err(anyhow::Error::new(e));
    }

    let ast = mcc::parse(&db, input);
    let diags: Vec<&Diagnostics> = mcc::parse::accumulated::<Diagnostics>(&db, input);
    if let ControlFlow::Break(ret) = cb.after_parse(&db, input, ast, diags) {
        return Outcome::EarlyReturn(ret);
    }

    let tacky = mcc::lowering::lower(&db, ast, input);
    let diags: Vec<&Diagnostics> =
        mcc::lowering::lower::accumulated::<Diagnostics>(&db, ast, input);
    if let ControlFlow::Break(ret) = cb.after_lower(&db, tacky, diags) {
        return Outcome::EarlyReturn(ret);
    }

    let program = mcc::codegen::generate_assembly(&db, tacky);
    let diags: Vec<&Diagnostics> =
        mcc::codegen::generate_assembly::accumulated::<Diagnostics>(&db, tacky);

    if let ControlFlow::Break(ret) = cb.after_codegen(&db, program, diags) {
        return Outcome::EarlyReturn(ret);
    }

    let assembly = match mcc::render::render_program(&db, program, target.clone()) {
        Ok(assembly) => assembly,
        Err(e) => return Outcome::Err(e.into()),
    };
    let diags: Vec<&Diagnostics> =
        mcc::render::render_program::accumulated::<Diagnostics>(&db, program, target.clone());

    if let ControlFlow::Break(ret) = cb.after_render_assembly(&db, assembly.clone(), diags) {
        return Outcome::EarlyReturn(ret);
    }

    let asm = temp.path().join("assembly.s");
    if let Err(e) = std::fs::write(&asm, assembly) {
        return Outcome::Err(e.into());
    }

    let output_path = output
        .clone()
        .unwrap_or_else(|| Path::new(input.path(&db)).with_extension(""));

    if let Err(e) =
        mcc::assemble_and_link(&db, cc.clone(), asm, output_path.clone(), target.clone())
    {
        return Outcome::Err(e.into());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        if let Err(e) = std::fs::set_permissions(&output_path, perms) {
            return Outcome::Err(e.into());
        }
    }

    if let ControlFlow::Break(ret) = cb.after_compile(&db, output_path) {
        return Outcome::EarlyReturn(ret);
    }

    Outcome::Ok
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
    type Output;

    /// Called after parsing the file.
    fn after_parse<'db>(
        &mut self,
        _db: &'db dyn mcc::Db,
        _source_file: mcc::types::SourceFile,
        _ast: Ast<'db>,
        _diags: Vec<&Diagnostics>,
    ) -> ControlFlow<Self::Output> {
        ControlFlow::Continue(())
    }

    /// Called after lowering the [Abstract Syntax Tree][Ast] to [Three Address Code][tacky].
    fn after_lower<'db>(
        &mut self,
        _db: &'db dyn mcc::Db,
        _tacky: tacky::Program<'db>,
        _diags: Vec<&Diagnostics>,
    ) -> ControlFlow<Self::Output> {
        ControlFlow::Continue(())
    }

    /// Called after generating [assembly instructions][asm] from [Three Address Code][tacky].
    fn after_codegen<'db>(
        &mut self,
        _db: &'db dyn mcc::Db,
        _asm: asm::Program<'db>,
        _diags: Vec<&Diagnostics>,
    ) -> ControlFlow<Self::Output> {
        ControlFlow::Continue(())
    }

    /// Called after rendering the [assembly instructions][asm] to a string.
    fn after_render_assembly(
        &mut self,
        _db: &dyn mcc::Db,
        _asm: Text,
        _diags: Vec<&Diagnostics>,
    ) -> ControlFlow<Self::Output> {
        ControlFlow::Continue(())
    }

    fn after_compile(&mut self, _db: &dyn mcc::Db, _binary: PathBuf) -> ControlFlow<Self::Output> {
        ControlFlow::Continue(())
    }
}
