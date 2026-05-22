mod cli;

use clap::Parser;

use crate::cli::Cli;

fn main() {
    let args = Cli::parse();
    println!("input:     {}", args.input.display());
    println!("in_place:  {}", args.in_place);
    println!("output:    {:?}", args.output);
    println!("overwrite: {}", args.overwrite);
}
