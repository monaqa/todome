use std::{io::Read, path::PathBuf};

use anyhow::*;

use structopt::StructOpt;
use todome::{
    format::{FormattedCst, FormattingOption},
    parser::Cst,
};

#[derive(Debug, Clone, StructOpt)]
struct Opts {
    #[structopt(subcommand)]
    subcmd: SubCmd,
}

#[derive(Debug, Clone, StructOpt)]
enum SubCmd {
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

            let cst = Cst::parse_source_file(&text)?;
            let formatted_cst = FormattedCst::from_cst(&cst);
            let opt = FormattingOption::new();
            let formatted = formatted_cst.to_formatted_string(&opt);

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
