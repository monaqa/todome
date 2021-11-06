use std::{io::Read, path::PathBuf};

use anyhow::*;

use structopt::StructOpt;
use todome::{
    format::format_lines,
    structure::syntax::{Cst, Document},
};

#[derive(Debug, Clone, StructOpt)]
struct Opts {
    #[structopt(subcommand)]
    subcmd: SubCmd,
}

#[derive(Debug, Clone, StructOpt)]
enum SubCmd {
    #[structopt(alias = "fmt")]
    Format {
        /// フォーマットの対象となるファイル。
        #[structopt(short, long)]
        input: Option<PathBuf>,
        #[structopt(long)]
        inplace: bool,
    },
}

fn main() -> Result<()> {
    let opts = Opts::from_args();

    match opts.subcmd {
        SubCmd::Format { ref input, inplace } => {
            let text = if let Some(input) = input {
                std::fs::read_to_string(input)?
            } else {
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf)?;
                buf
            };

            let formatted = format_lines(&text)?;

            if inplace && input.is_some() {
                let input = input.as_ref().unwrap();
                std::fs::write(input, formatted)?;
            } else {
                print!("{}", formatted);
            }
        }
    }

    Ok(())
}
