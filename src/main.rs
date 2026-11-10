mod index;
mod metadata;
mod scip_io;

use std::collections::HashSet;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context as _, Result, bail};
use clap::{Args, Parser, Subcommand};
use serde_json::json;
use tsrs_ast::{self as ast, Diagnostic, Kind, Node, SourceFile, Symbol};
use tsrs_compiler::{
    Checker, Context, Program, ProgramOptions, new_cached_fs_compiler_host, new_program,
};
use tsrs_core::P;
use tsrs_scanner::{get_ecma_line_and_utf16_character_of_position, get_token_pos_of_node};
use tsrs_tsoptions::{ParseConfigHost, get_parsed_command_line_of_config_file};
use tsrs_vfs::{FS, bundled, osvfs};

struct ConfigHost<'a> {
    fs: &'a dyn FS,
    root: &'a str,
}

impl ParseConfigHost for ConfigHost<'_> {
    fn fs(&self) -> &dyn FS {
        self.fs
    }

    fn get_current_directory(&self) -> &str {
        self.root
    }
}

struct Project {
    root: PathBuf,
    config: PathBuf,
    program: &'static Program,
    files: Vec<P<SourceFile>>,
}

impl Project {
    fn load(input: &ProjectInput, project_root: Option<&Path>) -> Result<Self> {
        let config = if input.project.is_dir() {
            input.project.join("tsconfig.json")
        } else {
            input.project.clone()
        };
        let config = config
            .canonicalize()
            .with_context(|| format!("reading {}", config.display()))?;
        let config_directory = config.parent().context("tsconfig has no parent")?;
        let root = project_root
            .map(Path::canonicalize)
            .transpose()?
            .unwrap_or_else(|| config_directory.to_owned());
        let root_text = config_directory
            .to_str()
            .context("project path is not UTF-8")?;
        let library = input.lib.as_deref().map(Path::canonicalize).transpose()?;
        let library = library
            .as_deref()
            .map(|path| path.to_str().context("library path is not UTF-8"))
            .transpose()?;
        let bundled_lib = bundled::lib_path();
        let fs = Arc::new(bundled::wrap_fs(osvfs::fs()));
        let config_host = ConfigHost {
            fs: &*fs,
            root: root_text,
        };
        let (parsed, read_errors) = get_parsed_command_line_of_config_file(
            config.to_str().context("config path is not UTF-8")?,
            None,
            None,
            &config_host,
            None,
        );
        print_diagnostics("config read", &read_errors);
        let parsed = parsed.context("tsrs could not read the config")?;
        print_diagnostics("config", &parsed.errors);
        if !parsed.errors.is_empty() {
            bail!("tsrs reported configuration errors");
        }
        let roots: HashSet<_> = parsed.file_names().iter().cloned().collect();
        let host = new_cached_fs_compiler_host(
            root_text,
            fs,
            library.unwrap_or(&bundled_lib),
            None,
            None,
            None,
        );
        let program = new_program(ProgramOptions::new(P::new(parsed), host));
        let files: Vec<_> = program
            .source_files()
            .iter()
            .copied()
            .filter(|file| roots.contains(file.file_name()))
            .collect();
        if files.len() != roots.len() || files.is_empty() {
            bail!(
                "tsconfig specified {} roots, but tsrs loaded {}",
                roots.len(),
                files.len()
            );
        }
        eprintln!(
            "loaded {} roots, {} files total",
            files.len(),
            program.source_files().len()
        );
        Ok(Self {
            root,
            config,
            program,
            files,
        })
    }
}

fn print_diagnostics(phase: &str, diagnostics: &[P<Diagnostic>]) {
    for diagnostic in diagnostics {
        let file = diagnostic.file().map_or("", |file| file.get().file_name());
        eprintln!(
            "{phase}: TS{} {file} {}",
            diagnostic.code(),
            diagnostic.message_text()
        );
    }
}

fn visit(node: P<Node>, visitor: &mut impl FnMut(P<Node>)) {
    visitor(node);
    node.for_each_child_static(&mut |child| {
        visit(child, visitor);
        false
    });
}

fn node_range(file: P<SourceFile>, node: P<Node>) -> scip_io::Range {
    let start = get_token_pos_of_node(node, file, false);
    let end = node.end().expect_source("parsed node has an end");
    source_range(file, start, end)
}

fn source_range(file: P<SourceFile>, start: u32, end: u32) -> scip_io::Range {
    let (sl, sc) = get_ecma_line_and_utf16_character_of_position(&*file, start);
    let (el, ec) = get_ecma_line_and_utf16_character_of_position(&*file, end);
    [sl as i32, sc as i32, el as i32, ec as i32]
}

fn parents(node: P<Node>) -> Vec<String> {
    std::iter::successors(node.parent(), |node| node.parent())
        .map(|node| format!("{:?}", node.kind()))
        .collect()
}

fn declarations(symbol: Option<P<Symbol>>) -> &'static [P<Node>] {
    symbol.map_or(&[], |symbol| symbol.declarations())
}

fn describe_declarations(nodes: &[P<Node>]) -> Vec<serde_json::Value> {
    nodes
        .iter()
        .map(|&decl| {
            let file = ast::get_source_file_of_node(decl);
            json!({
                "file": file.map(|file| file.get().file_name()),
                "kind": format!("{:?}", decl.kind()),
                "range": file.filter(|_| !decl.pos().is_synthetic() && !decl.end().is_synthetic())
                    .map(|file| node_range(file, decl)),
                "parents": parents(decl),
            })
        })
        .collect()
}

fn inspect(
    project: &Project,
    file: &Path,
    checker: &mut Checker,
    out: &mut dyn Write,
) -> Result<()> {
    let file = project.config.parent().unwrap().join(file).canonicalize()?;
    let source = project
        .program
        .get_source_file(file.to_str().context("file path is not UTF-8")?)
        .context("file is outside the loaded program")?;
    let mut result = Ok(());
    visit(source.as_node(), &mut |node| {
        if result.is_err() || !matches!(node.kind(), Kind::Identifier | Kind::PrivateIdentifier) {
            return;
        }
        let symbol = checker.get_symbol_at_location_exported(node);
        result = writeln!(
            out,
            "{}",
            json!({
                "identifier": node.text(), "range": node_range(source, node), "parents": parents(node),
                "symbol": symbol.map(|symbol| symbol.name()),
                "declarations": describe_declarations(declarations(symbol)),
            })
        );
    });
    result?;
    Ok(())
}

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a SCIP index.
    #[command(visible_alias = "scip")]
    Index {
        #[command(flatten)]
        input: ProjectInput,
        /// SCIP output path, relative to the current directory.
        #[arg(short, long, default_value = "index.scip")]
        output: PathBuf,
        /// Root for document paths. Defaults to the tsconfig directory.
        #[arg(long)]
        project_root: Option<PathBuf>,
        /// Emit occurrences and bare symbols only.
        #[arg(long)]
        minimal: bool,
        /// Skip emitting documents above this number of UTF-8 bytes. Zero disables the limit.
        #[arg(long, default_value_t = 1_000_000)]
        max_file_byte_size: usize,
    },
    /// Inspect checker symbols and their declarations in one file.
    Inspect {
        #[command(flatten)]
        input: ProjectInput,
        /// File relative to the tsconfig directory.
        #[arg(long)]
        file: PathBuf,
    },
    /// Print project, syntax, and semantic diagnostics.
    Diagnose {
        #[command(flatten)]
        input: ProjectInput,
    },
    /// Decode a SCIP file to JSON Lines of documents and occurrence tuples.
    Dump { index: PathBuf },
}

#[derive(Args)]
struct ProjectInput {
    /// A directory containing tsconfig.json, or the path to a config file.
    #[arg(default_value = ".")]
    project: PathBuf,
    /// Override the compiler's bundled TypeScript library directory.
    #[arg(long)]
    lib: Option<PathBuf>,
}

fn main() -> Result<()> {
    let ctx = Context::background();
    match Cli::parse().command {
        Command::Dump { index } => scip_io::dump(&index)?,
        Command::Diagnose { input } => {
            let project = Project::load(&input, None)?;
            let diagnostics = [
                ("program", project.program.get_program_diagnostics()),
                (
                    "syntax",
                    project.program.get_syntactic_diagnostics(&ctx, None),
                ),
                (
                    "semantic",
                    project.program.get_semantic_diagnostics(&ctx, None),
                ),
            ];
            for (phase, diagnostics) in &diagnostics {
                print_diagnostics(phase, diagnostics);
            }
            if diagnostics
                .iter()
                .any(|(_, diagnostics)| !diagnostics.is_empty())
            {
                bail!("TypeScript reported diagnostics");
            }
        }
        Command::Inspect { input, file } => {
            let project = Project::load(&input, None)?;
            let mut output = BufWriter::new(io::stdout().lock());
            inspect(
                &project,
                &file,
                &mut project.program.get_type_checker(&ctx),
                &mut output,
            )?;
            output.flush()?;
        }
        Command::Index {
            input,
            output,
            project_root,
            minimal,
            max_file_byte_size,
        } => {
            let project = Project::load(&input, project_root.as_deref())?;
            index::write_index(
                &project,
                &mut project.program.get_type_checker(&ctx),
                &output,
                index::Options {
                    minimal,
                    max_file_byte_size,
                },
            )?;
        }
    }
    Ok(())
}
