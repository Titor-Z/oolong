use std::path::Path;
use std::process::ExitCode;

use clap::Parser;
use oolong::runtime::{self, OolongRuntime};

/// OOLONG 🍵 — JS/TS Runtime
#[derive(Parser)]
#[command(name = "oolong", version, about)]
enum Cli {
    /// Run a JavaScript/TypeScript file as an ES Module
    Run {
        /// Entry point file (.js, .ts, .tsx, .mjs, .mts)
        file: String,
        /// Arguments passed to the script (after --)
        #[arg(last = true)]
        script_args: Vec<String>,
    },
    /// Evaluate inline JavaScript/TypeScript code
    Eval {
        /// Code to evaluate
        code: String,
    },
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    match cli {
        Cli::Run { file, script_args } => run_file(&file, &script_args),
        Cli::Eval { code } => eval_code(&code),
    }
}

fn run_file(file: &str, script_args: &[String]) -> ExitCode {
    let path = Path::new(file);
    if !path.exists() {
        eprintln!("error: file not found: {file}");
        return ExitCode::FAILURE;
    }

    let root = path.parent().unwrap_or(Path::new("."));
    let exe = std::env::args().next().unwrap_or_else(|| "oolong".into());
    let mut argv = vec![exe, file.to_string()];
    argv.extend_from_slice(script_args);

    runtime::set_cli_args(argv);

    match OolongRuntime::new(root).and_then(|mut rt| rt.eval_module_file(path)) {
        Ok(result) => {
            if !result.is_empty() && result != "undefined" {
                println!("{result}");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn eval_code(code: &str) -> ExitCode {
    let root = Path::new(".");
    let exe = std::env::args().next().unwrap_or_else(|| "oolong".into());
    let argv = vec![exe, String::new()];

    runtime::set_cli_args(argv);

    match OolongRuntime::new(root).and_then(|mut rt| rt.eval_module_str(code, None)) {
        Ok(result) => {
            if !result.is_empty() && result != "undefined" {
                println!("{result}");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}
