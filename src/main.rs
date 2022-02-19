use std::{io::Read, path::PathBuf};

use anyhow::*;

use clap::{Args, Parser, Subcommand};
use todome::subcmd::format::format_lines;

#[derive(Debug, Clone, Parser)]
#[clap()]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCmd,
}

#[derive(Debug, Clone, Subcommand)]
enum SubCmd {
    #[clap(alias = "fmt")]
    Format(InputInfo),
    Sort(InputInfo),
}

#[derive(Debug, Clone, Args)]
struct InputInfo {
    #[clap(short, long)]
    input: Option<PathBuf>,
    #[clap(long)]
    inplace: bool,
}

impl InputInfo {
    fn get_text(&self) -> Result<String> {
        let text = if let Some(input) = &self.input {
            std::fs::read_to_string(input)?
        } else {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            buf
        };
        Ok(text)
    }

    fn save_or_print_text(&self, text: &str) -> Result<()> {
        if self.inplace && self.input.is_some() {
            let input = self.input.as_ref().unwrap();
            std::fs::write(input, text)?;
        } else {
            print!("{text}");
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    match Opts::parse().subcmd {
        SubCmd::Format(input) => {
            let text = input.get_text()?;
            let formatted = format_lines(&text)?;
            input.save_or_print_text(&formatted)?;
        }
        SubCmd::Sort(input) => {
            let text = input.get_text()?;
            todo!()
            // let sorted = sort_tasks(&text)?;
            // input.save_or_print_text(&sorted)?;
        }
    }

    Ok(())
}
