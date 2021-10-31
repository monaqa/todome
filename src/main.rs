use anyhow::*;

use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
struct Opts {
    #[structopt(subcommand)]
    subcmd: SubCmd,
}

#[derive(Debug, Clone, StructOpt)]
enum SubCmd {}

fn main() -> Result<()> {
    let opts = Opts::from_args();

    Ok(())
}
