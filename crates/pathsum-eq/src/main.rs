use std::env;
use std::fs;
use std::process::ExitCode;

use pathsum_eq::{check, CheckRequest};

fn main() -> ExitCode {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|a| a == "--help" || a == "-h") || args.len() < 2 {
        eprintln!(
            "pathsum-eq LEFT.qasm RIGHT.qasm [--mode hybrid|reduction|wmc] [--counter all|enumerate|dpll|bucket|gpmc|ganak] [--precision BITS]\nPATHSUM_GPMC selects the complex-weight GPMC binary.\nPATHSUM_GANAK selects the GANAK binary (https://github.com/meelgroup/ganak)."
        );
        return ExitCode::from(2);
    }
    let left = args.remove(0);
    let right = args.remove(0);
    let mut req = CheckRequest {
        circuit1: read_qasm(&left),
        circuit2: read_qasm(&right),
        ..CheckRequest::default()
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" => {
                req.mode = args.get(i + 1).cloned().unwrap_or_default();
                i += 2;
            }
            "--counter" => {
                req.counter = args.get(i + 1).cloned().unwrap_or_default();
                i += 2;
            }
            "--precision" => {
                req.precision = args
                    .get(i + 1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(req.precision);
                i += 2;
            }
            other => {
                eprintln!("unknown argument {other}");
                return ExitCode::from(2);
            }
        }
    }
    match check(&req) {
        Ok(report) => {
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(1)
        }
    }
}

fn read_qasm(path: &str) -> String {
    if path == "-" {
        return String::new();
    }
    fs::read_to_string(path).unwrap_or_else(|err| {
        eprintln!("could not read {path}: {err}");
        String::new()
    })
}
