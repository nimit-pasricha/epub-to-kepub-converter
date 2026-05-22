use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use walkdir::WalkDir;

use crate::cli::Cli;

/// One conversion: read `source`, write the kepub to `target`.
#[derive(Debug, Clone)]
pub struct Job {
    pub source: PathBuf,
    pub target: PathBuf,
}

/// The full set of work resolved from the CLI arguments.
#[derive(Debug, Default)]
pub struct Plan {
    pub jobs: Vec<Job>,
    /// Sources skipped because their target already exists.
    pub skipped: Vec<PathBuf>,
}

fn is_epub(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("epub"))
        .unwrap_or(false)
}

/// True when the filename already ends in the kepub double-extension.
fn is_kepub(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.to_lowercase().ends_with(".kepub.epub"))
        .unwrap_or(false)
}

/// Target filename for a source: `foo.epub` -> `foo.kepub.epub`;
/// an already-kepub `foo.kepub.epub` keeps its name.
fn target_file_name(source: &Path) -> Result<String> {
    let name = source
        .file_name()
        .and_then(|n| n.to_str())
        .context("source path has no filename")?;
    if is_kepub(source) {
        return Ok(name.to_string());
    }
    let base = &name[..name.len() - ".epub".len()];
    Ok(format!("{base}.kepub.epub"))
}

/// Find every `.epub` to convert. A file input must itself be an `.epub`;
/// a directory is searched recursively, excluding already-converted kepubs.
fn discover_sources(input: &Path) -> Result<Vec<PathBuf>> {
    if !input.exists() {
        bail!("input path does not exist: {}", input.display());
    }
    if input.is_file() {
        if !is_epub(input) {
            bail!("input file is not an .epub: {}", input.display());
        }
        return Ok(vec![input.to_path_buf()]);
    }
    let mut sources = Vec::new();
    for entry in WalkDir::new(input).follow_links(false) {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type().is_file() && is_epub(path) && !is_kepub(path) {
            sources.push(path.to_path_buf());
        }
    }
    sources.sort();
    Ok(sources)
}

/// Resolve the CLI arguments into a concrete set of conversion jobs.
pub fn build_plan(cli: &Cli) -> Result<Plan> {
    let sources = discover_sources(&cli.input)?;
    if sources.is_empty() {
        bail!("no .epub files found at {}", cli.input.display());
    }

    let mut plan = Plan::default();
    let mut seen: HashMap<PathBuf, PathBuf> = HashMap::new();

    for source in sources {
        let file_name = target_file_name(&source)?;
        let target = match &cli.output {
            Some(dir) => dir.join(&file_name),
            None => source.with_file_name(&file_name),
        };

        if let Some(prev) = seen.insert(target.clone(), source.clone()) {
            bail!(
                "output collision: `{}` and `{}` both map to `{}`",
                prev.display(),
                source.display(),
                target.display()
            );
        }

        // Skip when the target already exists, unless --overwrite. Converting a
        // file onto itself (an explicit `.kepub.epub` input) is never a skip.
        if target != source && target.exists() && !cli.overwrite {
            plan.skipped.push(source);
            continue;
        }
        plan.jobs.push(Job { source, target });
    }

    Ok(plan)
}
