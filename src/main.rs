mod cli;
mod epub;
mod html;
mod jobs;

use anyhow::Result;
use clap::Parser;

use crate::cli::Cli;

fn main() -> Result<()> {
    let args = Cli::parse();
    let plan = jobs::build_plan(&args)?;

    println!("{} file(s) to convert:", plan.jobs.len());
    for job in &plan.jobs {
        println!("  {}  ->  {}", job.source.display(), job.target.display());
    }
    if !plan.skipped.is_empty() {
        println!("{} file(s) skipped (target exists):", plan.skipped.len());
        for source in &plan.skipped {
            println!("  {}", source.display());
        }
    }
    Ok(())
}
