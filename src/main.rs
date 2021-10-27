use std::path::PathBuf;

use anyhow::*;
use todome::parser;

use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
struct Opts {
    input: PathBuf,
}

fn main() -> Result<()> {
    let opts = Opts::from_args();
    let text = std::fs::read_to_string(&opts.input)?;
    let cst = parser::parse_to_cst(&text)?;
    println!("{}", cst);
    println!("{:#?}", cst);
    Ok(())
}
