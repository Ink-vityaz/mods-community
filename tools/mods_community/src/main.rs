//! `mods-community`: checks the mods handed in (README.md).

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
usage: mods-community check [--root <folder>] [--only <id>] [--changed <file>]

  --root      the folder holding mods/ (default: this one)
  --only      one mod
  --changed   a file listing the paths a contribution changes, one a line: each must be in mods/<id>/
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut root = PathBuf::from(".");
    let mut only = None;
    let mut changed = None;
    if args.first().map(String::as_str) != Some("check") || args.len() % 2 == 0 {
        print!("{USAGE}");
        return ExitCode::FAILURE;
    }
    for pair in args[1..].chunks(2) {
        match pair[0].as_str() {
            "--root" => root = PathBuf::from(&pair[1]),
            "--only" => only = Some(pair[1].clone()),
            "--changed" => changed = Some(pair[1].clone()),
            _ => {
                print!("{USAGE}");
                return ExitCode::FAILURE;
            }
        }
    }
    let mut problems = mods_community::check(&root, only.as_deref());
    if let Some(file) = changed {
        match std::fs::read_to_string(&file) {
            Ok(text) => problems.extend(mods_community::outside(&text).into_iter().map(|path| format!("{path}: a mod changes only its own folder in mods/"))),
            Err(error) => problems.push(format!("{file}: {error}")),
        }
    }
    for problem in &problems {
        println!("{problem}");
    }
    if problems.is_empty() {
        println!("every mod is all right");
        return ExitCode::SUCCESS;
    }
    println!("{} to mend", problems.len());
    ExitCode::FAILURE
}
