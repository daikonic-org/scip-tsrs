use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context as _, Result, bail};
use tsrs_ast::SourceFile;
use tsrs_compiler::{
    Program, ProgramOptions, ProgramRegions, new_cached_fs_compiler_host, new_program,
};
use tsrs_core::{P, resolve_project_reference_path, tspath};
use tsrs_tsoptions::{
    ParseConfigHost, ParsedCommandLine, get_parsed_command_line_of_config_file,
    parse_config_file_text_to_json, parse_json_config_file_content,
};
use tsrs_vfs::{FS, bundled, osvfs};

use crate::{ProjectInput, print_diagnostics};

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

pub struct Project {
    pub root: PathBuf,
    pub config: PathBuf,
    pub program: &'static Program,
    pub files: Vec<P<SourceFile>>,
}

pub fn config_path(input: &ProjectInput) -> Result<PathBuf> {
    let config = if input.project.is_dir() {
        input.project.canonicalize()?.join("tsconfig.json")
    } else {
        input.project.clone()
    };
    if config.exists() {
        Ok(config.canonicalize()?)
    } else if input.infer_tsconfig && input.project.is_dir() {
        Ok(config)
    } else {
        bail!("reading {}: config does not exist", config.display())
    }
}

fn parse(config: &Path, fs: &dyn FS, infer: bool) -> Result<ParsedCommandLine> {
    let directory = config.parent().context("tsconfig has no parent")?;
    let root = directory.to_str().context("project path is not UTF-8")?;
    let name = config.to_str().context("config path is not UTF-8")?;
    let host = ConfigHost { fs, root };
    let parsed = if !config.exists() && infer {
        // Let the compiler apply its normal file discovery and excludes. No
        // inferred config is written into the project being indexed.
        let (json, errors) = parse_config_file_text_to_json(
            name,
            tspath::to_path(name, root, fs.use_case_sensitive_file_names()),
            r#"{"compilerOptions":{"allowJs":true,"checkJs":false,"target":"ESNext","module":"NodeNext","noEmit":true},"exclude":["node_modules",".git",".infigraph","dist","build","target","scratchpad"]}"#,
        );
        print_diagnostics("inferred config", &errors);
        parse_json_config_file_content(json, &host, root, None, name, &[], None)
    } else {
        let (parsed, errors) =
            get_parsed_command_line_of_config_file(name, None, None, &host, None);
        print_diagnostics("config read", &errors);
        parsed.context("tsrs could not read the config")?
    };
    print_diagnostics("config", &parsed.errors);
    if !parsed.errors.is_empty() {
        bail!("tsrs reported configuration errors in {name}");
    }
    Ok(parsed)
}

/// Compiler-parsed project references, canonicalized and visited once. A
/// solution config with no roots is a container, not an empty-project error.
pub fn configurations(input: &ProjectInput, follow_references: bool) -> Result<Vec<PathBuf>> {
    let mut pending = vec![config_path(input)?];
    let mut seen = HashSet::new();
    let mut configs = Vec::new();
    let fs = bundled::wrap_fs(osvfs::fs());
    while let Some(config) = pending.pop() {
        if !seen.insert(config.clone()) {
            continue;
        }
        let parsed = parse(&config, &fs, input.infer_tsconfig)?;
        if follow_references {
            // Reverse preserves the config's source order with the stack.
            for reference in parsed.project_references().iter().rev() {
                let path = PathBuf::from(resolve_project_reference_path(reference));
                pending.push(path.canonicalize().with_context(|| {
                    format!(
                        "project reference {} from {}",
                        path.display(),
                        config.display()
                    )
                })?);
            }
        }
        if !parsed.file_names().is_empty() {
            configs.push(config);
        }
    }
    if configs.is_empty() {
        bail!("no source files in the selected project graph");
    }
    Ok(configs)
}

impl Project {
    pub fn load(input: &ProjectInput, project_root: Option<&Path>) -> Result<Self> {
        Self::load_config(input, &config_path(input)?, project_root)
    }

    pub fn load_config(
        input: &ProjectInput,
        config: &Path,
        project_root: Option<&Path>,
    ) -> Result<Self> {
        let directory = config.parent().context("tsconfig has no parent")?;
        let root = project_root
            .map(Path::canonicalize)
            .transpose()?
            .unwrap_or_else(|| directory.to_owned());
        let library = input.lib.as_deref().map(Path::canonicalize).transpose()?;
        let library = library
            .as_deref()
            .map(|path| path.to_str().context("library path is not UTF-8"))
            .transpose()?;
        let fs = Arc::new(bundled::wrap_fs(osvfs::fs()));
        let parsed = parse(config, &*fs, input.infer_tsconfig)?;
        let roots: HashSet<_> = parsed.file_names().iter().cloned().collect();
        let host = new_cached_fs_compiler_host(
            directory.to_str().context("project path is not UTF-8")?,
            fs,
            library.unwrap_or(&bundled::lib_path()),
            None,
            None,
            None,
        );
        let mut options = ProgramOptions::new(P::new(parsed), host);
        options.use_source_of_project_reference = true;
        options.checker_regions = true;
        options.checker_diagnostic_regions = true;
        options.regions = Some(Arc::new(ProgramRegions::default()));
        let program = new_program(options);
        let files: Vec<_> = program
            .source_files()
            .iter()
            .copied()
            .filter(|file| roots.contains(file.file_name()))
            .collect();
        let project = Self {
            root,
            config: config.to_owned(),
            program,
            files,
        };
        if project.files.len() != roots.len() || project.files.is_empty() {
            bail!(
                "tsconfig specified {} roots, but tsrs loaded {}",
                roots.len(),
                project.files.len()
            );
        }
        eprintln!(
            "loaded {} roots, {} files total from {}",
            project.files.len(),
            program.source_files().len(),
            config.display()
        );
        Ok(project)
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        // SAFETY: each Project owns one new_program, never calls update_program
        // or shares its host, and all checkers/metadata are dropped before it.
        // P<SourceFile> is a non-owning pointer and is never read after drop.
        unsafe { tsrs_compiler::free_unshared_program(self.program) };
    }
}
