//! The `skopos` command line tool for OpenSkopos documents.
//!
//! The tool reads and writes files. `skopos-core` does all the other work.

use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use skopos_core::{assign_ids, check, parse, write, Format, Problem, Value};

#[derive(Parser)]
#[command(
    name = "skopos",
    version,
    about = "Check, format and convert OpenSkopos documents"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check documents against the schema and the rules of the specification.
    ///
    /// The exit code is 1 if a document has a problem.
    Check {
        /// JSON or YAML files (.json, .yaml or .yml)
        #[arg(required = true)]
        files: Vec<PathBuf>,
        /// Write the problems as JSON
        #[arg(long)]
        json: bool,
    },
    /// Write documents again in the canonical key order and layout.
    Fmt {
        /// JSON or YAML files. Each file keeps its format.
        #[arg(required = true)]
        files: Vec<PathBuf>,
        /// Change no files. Name each file that is not in the canonical
        /// layout, and exit with 1 if there is one.
        #[arg(long)]
        check: bool,
    },
    /// Convert a document between JSON and YAML.
    Convert {
        /// A JSON or YAML file
        file: PathBuf,
        /// The format to write
        #[arg(long, value_enum)]
        to: Target,
        /// The file to write. Without it, the document goes to standard output.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Give an ID to each item that has none, and update id_counters.
    ///
    /// An item has no ID if its `id` is absent, null or empty. The IDs come
    /// from `id_counters`, as in section 7 of the specification. A changed
    /// file is written in the canonical layout.
    Ids {
        /// JSON or YAML files
        #[arg(required = true)]
        files: Vec<PathBuf>,
        /// Change no files. Name each file with an item that needs an ID or a
        /// counter that is too low, and exit with 1 if there is one.
        #[arg(long)]
        check: bool,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Target {
    Json,
    Yaml,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let ok = match cli.command {
        Command::Check { files, json } => run_check(&files, json),
        Command::Fmt { files, check } => run_fmt(&files, check),
        Command::Convert { file, to, output } => run_convert(&file, to, output.as_deref()),
        Command::Ids { files, check } => run_ids(&files, check),
    };
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// The format of a file, from its name.
fn format_of(path: &Path) -> Result<Format, String> {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("json") => Ok(Format::Json),
        Some("yaml") | Some("yml") => Ok(Format::Yaml),
        Some("md") => Err("this version of skopos cannot read Markdown yet".to_string()),
        _ => Err("the file name must end with .json, .yaml or .yml".to_string()),
    }
}

/// Reads a file into its format, its text and its value.
fn read(path: &Path) -> Result<(Format, String, Value), Problem> {
    let format = format_of(path).map_err(|e| Problem::new("", e))?;
    let text = fs::read_to_string(path)
        .map_err(|e| Problem::new("", format!("cannot read the file: {e}")))?;
    let doc = parse(&text, format)?;
    Ok((format, text, doc))
}

/// Writes text to standard output. If the reader closed the pipe, for example
/// `skopos convert ... | head`, the tool stops writing with no error.
fn emit(text: &str) -> bool {
    match io::stdout().lock().write_all(text.as_bytes()) {
        Ok(()) => true,
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => true,
        Err(e) => {
            eprintln!("cannot write to standard output: {e}");
            false
        }
    }
}

fn report(path: &Path, problem: &Problem) {
    eprintln!("{}: {problem}", path.display());
}

fn write_file(path: &Path, text: &str) -> bool {
    match fs::write(path, text) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("{}: cannot write the file: {e}", path.display());
            false
        }
    }
}

#[derive(Serialize)]
struct FileProblems {
    file: String,
    problems: Vec<Problem>,
}

fn run_check(files: &[PathBuf], json: bool) -> bool {
    let results: Vec<FileProblems> = files
        .iter()
        .map(|path| FileProblems {
            file: path.display().to_string(),
            problems: match read(path) {
                Ok((_, _, doc)) => check(&doc),
                Err(problem) => vec![problem],
            },
        })
        .collect();
    let with_problems = results.iter().filter(|r| !r.problems.is_empty()).count();

    if json {
        let mut text = serde_json::to_string_pretty(&results).expect("problems serialize");
        text.push('\n');
        emit(&text);
    } else {
        let lines: String = results
            .iter()
            .flat_map(|r| r.problems.iter().map(move |p| format!("{}: {p}\n", r.file)))
            .collect();
        emit(&lines);
        let count: usize = results.iter().map(|r| r.problems.len()).sum();
        if count == 0 {
            eprintln!("{}: no problems.", plural(files.len(), "file"));
        } else {
            eprintln!(
                "{} in {} of {}.",
                plural(count, "problem"),
                with_problems,
                plural(files.len(), "file")
            );
        }
    }
    with_problems == 0
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}

fn run_fmt(files: &[PathBuf], check_only: bool) -> bool {
    let mut ok = true;
    for path in files {
        let (format, old_text, doc) = match read(path) {
            Ok(r) => r,
            Err(problem) => {
                report(path, &problem);
                ok = false;
                continue;
            }
        };
        let text = write(&doc, format);
        if text == old_text {
            continue;
        }
        if check_only {
            emit(&format!("{}\n", path.display()));
            ok = false;
        } else {
            ok &= write_file(path, &text);
        }
    }
    ok
}

fn run_convert(file: &Path, to: Target, output: Option<&Path>) -> bool {
    let doc = match read(file) {
        Ok((_, _, doc)) => doc,
        Err(problem) => {
            report(file, &problem);
            return false;
        }
    };
    let format = match to {
        Target::Json => Format::Json,
        Target::Yaml => Format::Yaml,
    };
    let text = write(&doc, format);
    match output {
        Some(path) => write_file(path, &text),
        None => emit(&text),
    }
}

fn run_ids(files: &[PathBuf], check_only: bool) -> bool {
    let mut ok = true;
    for path in files {
        let (format, _, mut doc) = match read(path) {
            Ok(r) => r,
            Err(problem) => {
                report(path, &problem);
                ok = false;
                continue;
            }
        };
        let report_ids = match assign_ids(&mut doc) {
            Ok(r) => r,
            Err(problems) => {
                for p in &problems {
                    report(path, p);
                }
                ok = false;
                continue;
            }
        };
        if !report_ids.changed() {
            continue;
        }
        let mut changes: Vec<String> = report_ids
            .assigned
            .iter()
            .map(|a| format!("/items/{} gets {}", a.index, a.id))
            .collect();
        if report_ids.counters_changed {
            changes.push("id_counters changes".to_string());
        }
        emit(&format!("{}: {}\n", path.display(), changes.join(", ")));
        if check_only {
            ok = false;
        } else {
            ok &= write_file(path, &write(&doc, format));
        }
    }
    ok
}
