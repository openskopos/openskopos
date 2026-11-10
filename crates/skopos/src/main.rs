//! The `skopos` command line tool for OpenSkopos documents.
//!
//! The tool reads and writes files. `skopos-core` does all the other work.

use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use skopos_core::{assign_ids, check, markdown, parse, write, Format, Problem, Value};

#[derive(Parser)]
#[command(
    name = "skopos",
    version,
    about = "Check, format, convert and import OpenSkopos documents"
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
        /// JSON, YAML or Markdown files (.json, .yaml, .yml or .md)
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
    /// Convert a document between JSON, YAML and Markdown.
    Convert {
        /// A JSON, YAML or Markdown file
        file: PathBuf,
        /// The format to write
        #[arg(long, value_enum)]
        to: Target,
        /// The file to write. Without it, the document goes to standard output.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// For Markdown: another document to copy material from, such as a
        /// pinned glossary (section 10.7 of the specification). Give it once
        /// for each document.
        #[arg(long = "input", value_name = "FILE")]
        inputs: Vec<PathBuf>,
    },
    /// Read an edited Markdown file of a stored version, and write the
    /// version to store (section 10.8 of the specification).
    ///
    /// If the stored version is final and the content changed, the result is
    /// the next version as a draft. If it is final and the content did not
    /// change, the result is the stored version. If it is a draft, the file
    /// replaces it and it keeps its version number.
    Import {
        /// The edited Markdown file
        file: PathBuf,
        /// The stored version, as JSON or YAML
        #[arg(long, value_name = "FILE")]
        stored: PathBuf,
        /// The file to write, as JSON or YAML. Without it, the document goes
        /// to standard output in the format of the stored version.
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
    #[value(alias = "md")]
    Markdown,
}

/// The format of a file.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FileFormat {
    Data(Format),
    Markdown,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let ok = match cli.command {
        Command::Check { files, json } => run_check(&files, json),
        Command::Fmt { files, check } => run_fmt(&files, check),
        Command::Convert {
            file,
            to,
            output,
            inputs,
        } => run_convert(&file, to, output.as_deref(), &inputs),
        Command::Import {
            file,
            stored,
            output,
        } => run_import(&file, &stored, output.as_deref()),
        Command::Ids { files, check } => run_ids(&files, check),
    };
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// The format of a file, from its name.
fn format_of(path: &Path) -> Result<FileFormat, Problem> {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("json") => Ok(FileFormat::Data(Format::Json)),
        Some("yaml") | Some("yml") => Ok(FileFormat::Data(Format::Yaml)),
        Some("md") | Some("markdown") => Ok(FileFormat::Markdown),
        _ => Err(Problem::new(
            "",
            "the file name must end with .json, .yaml, .yml or .md",
        )),
    }
}

/// The format of a JSON or YAML file, from its name.
fn data_format_of(path: &Path) -> Result<Format, Problem> {
    match format_of(path)? {
        FileFormat::Data(format) => Ok(format),
        FileFormat::Markdown => Err(Problem::new(
            "",
            "this command works on JSON and YAML files. To read Markdown, use skopos convert or skopos import",
        )),
    }
}

/// Reads a JSON or YAML file into its format, its text and its value.
fn read(path: &Path) -> Result<(Format, String, Value), Problem> {
    let format = data_format_of(path)?;
    let text = read_text(path)?;
    let doc = parse(&text, format)?;
    Ok((format, text, doc))
}

fn read_text(path: &Path) -> Result<String, Problem> {
    fs::read_to_string(path).map_err(|e| Problem::new("", format!("cannot read the file: {e}")))
}

/// Reads a JSON, YAML or Markdown file into a document. The reader of
/// Markdown checks the document, so its errors include the problems that
/// `check` finds. Warnings go to standard error.
fn read_any(path: &Path) -> Result<Value, Vec<Problem>> {
    match format_of(path).map_err(|p| vec![p])? {
        FileFormat::Data(format) => {
            let text = read_text(path).map_err(|p| vec![p])?;
            parse(&text, format).map_err(|p| vec![p])
        }
        FileFormat::Markdown => {
            let text = read_text(path).map_err(|p| vec![p])?;
            let name = path.file_name().and_then(|n| n.to_str());
            match markdown::read(&text, name) {
                Ok((doc, warnings)) => {
                    for w in &warnings {
                        eprintln!("{}: warning: {w}", path.display());
                    }
                    Ok(doc)
                }
                Err(errors) => Err(errors.iter().map(markdown_problem).collect()),
            }
        }
    }
}

/// A Markdown error as a problem. The message starts with the line.
fn markdown_problem(e: &markdown::Error) -> Problem {
    let line = e.line.map(|l| format!("line {l}: ")).unwrap_or_default();
    match &e.problem {
        Some(p) => Problem {
            message: format!("{line}{}", p.message),
            ..p.clone()
        },
        None => Problem::new("", format!("{line}{}", e.message)),
    }
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
            problems: match read_any(path) {
                Ok(doc) => check(&doc),
                Err(problems) => problems,
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

fn run_convert(file: &Path, to: Target, output: Option<&Path>, inputs: &[PathBuf]) -> bool {
    let Some(doc) = read_or_report(file) else {
        return false;
    };
    let text = match to {
        Target::Json => write(&doc, Format::Json),
        Target::Yaml => write(&doc, Format::Yaml),
        Target::Markdown => {
            let mut others = Vec::with_capacity(inputs.len());
            for path in inputs {
                others.push(match read_or_report(path) {
                    Some(doc) => doc,
                    None => return false,
                });
            }
            markdown::write(&doc, &others)
        }
    };
    match output {
        Some(path) => write_file(path, &text),
        None => emit(&text),
    }
}

/// Reads a file, or reports its problems and gives `None`.
fn read_or_report(path: &Path) -> Option<Value> {
    match read_any(path) {
        Ok(doc) => Some(doc),
        Err(problems) => {
            for p in &problems {
                report(path, p);
            }
            None
        }
    }
}

fn run_import(file: &Path, stored_path: &Path, output: Option<&Path>) -> bool {
    if format_of(file).ok() != Some(FileFormat::Markdown) {
        report(file, &Problem::new("", "the file name must end with .md"));
        return false;
    }
    let (stored_format, _, stored) = match read(stored_path) {
        Ok(r) => r,
        Err(problem) => {
            report(stored_path, &problem);
            return false;
        }
    };
    let format = match output.map(data_format_of) {
        Some(Ok(format)) => format,
        Some(Err(problem)) => {
            report(output.expect("given"), &problem);
            return false;
        }
        None => stored_format,
    };
    let Some(read_doc) = read_or_report(file) else {
        return false;
    };
    let doc = markdown::next_version(&stored, &read_doc);
    let problems = check(&doc);
    if !problems.is_empty() {
        for p in &problems {
            report(file, p);
        }
        return false;
    }
    let version = |d: &Value| d.get("version").map(Value::to_string).unwrap_or_default();
    let what = if doc == stored {
        format!(
            "no change: version {} is final and its content is the same",
            version(&stored)
        )
    } else if doc.get("version") == stored.get("version") {
        format!("the file replaces draft version {}", version(&doc))
    } else {
        format!("new draft version {}", version(&doc))
    };
    eprintln!("{}: {what}", file.display());
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
