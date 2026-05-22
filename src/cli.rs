use std::path::PathBuf;

use clap::Parser;

/// Recursively convert EPUB files into Kobo kepub format.
#[derive(Parser, Debug)]
#[command(name = "kepub", version, about)]
pub struct Cli {
    /// EPUB file, or a directory to search recursively for `.epub` files.
    pub input: PathBuf,

    /// Replace each source `.epub` with its converted `.kepub.epub`
    /// (the original file is deleted).
    #[arg(long, conflicts_with = "output")]
    pub in_place: bool,

    /// Write every converted file into this single flat directory.
    #[arg(long, short, value_name = "DIR")]
    pub output: Option<PathBuf>,

    /// Re-convert files even if the target `.kepub.epub` already exists.
    #[arg(long)]
    pub overwrite: bool,
}
