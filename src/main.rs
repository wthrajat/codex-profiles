mod app;
mod cli;
mod codex;
mod config;
mod error;
mod paths;
mod permissions;
mod profile;

use clap::Parser;

use crate::cli::Cli;

fn main() {
    let cli = Cli::parse();

    let code = match app::run(cli) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            1
        }
    };
    if code != 0 {
        std::process::exit(code);
    }
}
