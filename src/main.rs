use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use dtl::{check_program, parse_program};

#[derive(Debug, Parser)]
#[command(name = "dtl")]
#[command(about = "Domain Typed Lisp checker")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Check { file: PathBuf },
}

fn main() {
    let cli = Cli::parse();
    let exit_code = match cli.command {
        Command::Check { file } => run_check(&file),
    };
    std::process::exit(exit_code);
}

fn run_check(file: &PathBuf) -> i32 {
    let src = match fs::read_to_string(file) {
        Ok(src) => src,
        Err(err) => {
            eprintln!("E-IO: failed to read {}: {}", file.display(), err);
            return 1;
        }
    };

    let program = match parse_program(&src) {
        Ok(p) => p,
        Err(diags) => {
            for d in diags {
                eprintln!("{d}");
            }
            return 1;
        }
    };

    match check_program(&program) {
        Ok(_) => {
            println!("ok");
            0
        }
        Err(diags) => {
            for d in diags {
                eprintln!("{d}");
            }
            1
        }
    }
}
