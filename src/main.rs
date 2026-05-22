mod cli;
mod convert;
mod epub;
mod html;
mod jobs;

use std::fs;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;

use crate::cli::Cli;
use crate::jobs::Job;

fn main() -> ExitCode {
    let args = Cli::parse();
    match run(&args) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

/// Run every conversion job, returning the number of failures.
fn run(args: &Cli) -> Result<usize> {
    let plan = jobs::build_plan(args)?;

    if let Some(dir) = &args.output {
        fs::create_dir_all(dir)
            .with_context(|| format!("creating output directory {}", dir.display()))?;
    }

    for source in &plan.skipped {
        println!("skip   {} (target exists)", source.display());
    }

    let bar = ProgressBar::new(plan.jobs.len() as u64);
    bar.set_style(
        ProgressStyle::with_template("{bar:40.cyan/blue} {pos}/{len} {wide_msg}")
            .expect("valid progress template"),
    );

    // Convert books in parallel; each job is fully independent.
    let results: Vec<(&Job, Result<()>)> = plan
        .jobs
        .par_iter()
        .map(|job| {
            let outcome = convert_job(job, args.in_place);
            bar.set_message(file_label(job));
            bar.inc(1);
            (job, outcome)
        })
        .collect();
    bar.finish_and_clear();

    let mut converted = 0usize;
    let mut failed = 0usize;
    for (job, outcome) in &results {
        match outcome {
            Ok(()) => converted += 1,
            Err(e) => {
                failed += 1;
                eprintln!("error: {}: {e:#}", job.source.display());
            }
        }
    }

    println!(
        "converted {converted}, skipped {}, failed {failed}",
        plan.skipped.len()
    );
    Ok(failed)
}

fn file_label(job: &Job) -> String {
    job.source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Convert a single EPUB and write its kepub, deleting the original for
/// `--in-place`.
fn convert_job(job: &Job, in_place: bool) -> Result<()> {
    let input = fs::read(&job.source)
        .with_context(|| format!("reading {}", job.source.display()))?;
    let output = convert::convert_epub(&input)
        .with_context(|| format!("converting {}", job.source.display()))?;
    fs::write(&job.target, &output)
        .with_context(|| format!("writing {}", job.target.display()))?;
    if in_place && job.source != job.target {
        fs::remove_file(&job.source)
            .with_context(|| format!("removing original {}", job.source.display()))?;
    }
    Ok(())
}
