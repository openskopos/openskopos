//! The commands of the `skopos` command line tool.
//!
//! [`run`] reads the arguments, runs one command and gives the exit code.
//! It does not touch files or the terminal itself: it asks a [`Host`] to do
//! that. `skopos-core` does all the other work.
//!
//! Two programs use this library with the same arguments, output and exit
//! codes:
//!
//! - the `skopos` binary of this crate, with a host that uses the standard
//!   library;
//! - the `skopos` command of the npm package `@openskopos/cli`, through the
//!   WebAssembly build in `crates/skopos-wasm`, with a host that uses Node.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use skopos_core::export::{self, Options, Tool};
use skopos_core::markdown::Fill;
use skopos_core::{assign_ids, check, diff, markdown, parse, write, Format, Problem, Value};

/// The file system and the terminal, as the commands see them.
///
/// Each method does one simple operation, with no logic of its own. An
/// error from a method is shown to the user with its `Display` text.
pub trait Host {
    /// Reads the bytes of a file.
    fn read(&mut self, path: &Path) -> io::Result<Vec<u8>>;
    /// Writes a file. The file is made if it does not exist, and replaced if
    /// it does.
    fn write(&mut self, path: &Path, text: &str) -> io::Result<()>;
    /// True if a file or folder exists at the path.
    fn exists(&mut self, path: &Path) -> bool;
    /// True if the path is a folder.
    fn is_dir(&mut self, path: &Path) -> bool;
    /// The names of the files and folders in a folder, in any order.
    fn read_dir(&mut self, path: &Path) -> io::Result<Vec<String>>;
    /// Makes a folder and the folders above it that do not exist.
    fn create_dir_all(&mut self, path: &Path) -> io::Result<()>;
    /// Removes a file.
    fn remove_file(&mut self, path: &Path) -> io::Result<()>;
    /// Removes an empty folder.
    fn remove_dir(&mut self, path: &Path) -> io::Result<()>;
    /// Writes text to standard output.
    fn stdout(&mut self, text: &str) -> io::Result<()>;
    /// Writes text to standard error.
    fn stderr(&mut self, text: &str);
    /// True if text for this stream can have colors.
    fn color(&mut self, stream: Stream) -> bool;
    /// The current time and a new artifact ID, for a Markdown file that has
    /// neither.
    fn fill(&mut self) -> Fill;
}

/// An output stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    Stdout,
    Stderr,
}

#[derive(Parser)]
#[command(
    name = "skopos",
    version,
    about = "Check, format, convert, import, export and compare OpenSkopos documents"
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
    /// Export a document to the files of a spec-driven development tool.
    ///
    /// Only accepted items are exported. Each item keeps its reference
    /// string. Items with no place in the tool are named in a "Not exported"
    /// note. See docs/exporters.md.
    Export {
        /// The document to export
        file: PathBuf,
        /// The tool
        #[arg(long, value_enum)]
        to: ExportTool,
        /// The root folder of the project. The tool's folders go inside it.
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
        /// The name of the change or feature. Without it, the name comes from
        /// the title.
        #[arg(long)]
        name: Option<String>,
        /// Spec Kit only: the feature number. Without it, skopos uses the
        /// number of the feature folder with the same name, or the next free
        /// number in specs/.
        #[arg(long)]
        number: Option<u32>,
        /// Write over files that exist
        #[arg(long)]
        force: bool,
        /// Write no files. Name the files and the items that are not exported.
        #[arg(long)]
        dry_run: bool,
    },
    /// Show the items that were added, changed, dropped and superseded
    /// between two versions, by item ID.
    Diff {
        /// The old version
        old: PathBuf,
        /// The new version
        new: PathBuf,
        /// Write the difference as JSON
        #[arg(long)]
        json: bool,
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

#[derive(Clone, Copy, ValueEnum)]
enum ExportTool {
    #[value(name = "openspec")]
    OpenSpec,
    Kiro,
    #[value(name = "speckit")]
    SpecKit,
}

impl From<ExportTool> for Tool {
    fn from(t: ExportTool) -> Tool {
        match t {
            ExportTool::OpenSpec => Tool::OpenSpec,
            ExportTool::Kiro => Tool::Kiro,
            ExportTool::SpecKit => Tool::SpecKit,
        }
    }
}

/// Runs the tool with its arguments. The first argument is the name of the
/// program. The result is the exit code: 0 if all went well, 1 if a command
/// found a problem, and 2 if the arguments are wrong.
pub fn run<I, T>(args: I, host: &mut dyn Host) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        // Help, the version and wrong arguments.
        Err(e) => {
            let stream = if e.use_stderr() {
                Stream::Stderr
            } else {
                Stream::Stdout
            };
            let rendered = e.render();
            let text = if host.color(stream) {
                rendered.ansi().to_string()
            } else {
                rendered.to_string()
            };
            match stream {
                Stream::Stdout => {
                    emit(host, &text);
                }
                Stream::Stderr => host.stderr(&text),
            }
            return u8::try_from(e.exit_code()).unwrap_or(2);
        }
    };
    let mut cmd = Commands { host };
    let ok = match cli.command {
        Command::Check { files, json } => cmd.check(&files, json),
        Command::Fmt { files, check } => cmd.fmt(&files, check),
        Command::Convert {
            file,
            to,
            output,
            inputs,
        } => cmd.convert(&file, to, output.as_deref(), &inputs),
        Command::Import {
            file,
            stored,
            output,
        } => cmd.import(&file, &stored, output.as_deref()),
        Command::Ids { files, check } => cmd.ids(&files, check),
        Command::Export {
            file,
            to,
            output,
            name,
            number,
            force,
            dry_run,
        } => cmd.export(
            &file,
            to.into(),
            &output,
            Options {
                name,
                number,
                existing: Vec::new(),
            },
            force,
            dry_run,
        ),
        Command::Diff { old, new, json } => cmd.diff(&old, &new, json),
    };
    if ok {
        0
    } else {
        1
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
fn emit(host: &mut dyn Host, text: &str) -> bool {
    match host.stdout(text) {
        Ok(()) => true,
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => true,
        Err(e) => {
            host.stderr(&format!("cannot write to standard output: {e}\n"));
            false
        }
    }
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}

/// The sequence number of a Spec Kit feature folder such as `004-billing`:
/// three or more digits and `-`. A folder with a timestamp prefix, such as
/// `20261110-093000-login`, has no sequence number. Spec Kit reads the
/// folders in the same way.
fn speckit_folder_number(folder: &str) -> Option<(u32, &str)> {
    let digits = folder.bytes().take_while(u8::is_ascii_digit).count();
    let rest = folder[digits..].strip_prefix('-')?;
    if digits < 3 {
        return None;
    }
    let time = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 8 && time == 6 && rest[time..].starts_with('-') {
        return None;
    }
    Some((folder[..digits].parse().ok()?, rest))
}

#[derive(Serialize)]
struct FileProblems {
    file: String,
    problems: Vec<Problem>,
}

/// The commands, with the host that they use.
struct Commands<'h> {
    host: &'h mut dyn Host,
}

impl Commands<'_> {
    fn out(&mut self, text: &str) -> bool {
        emit(self.host, text)
    }

    /// Writes one line to standard error.
    fn err(&mut self, line: &str) {
        self.host.stderr(&format!("{line}\n"));
    }

    fn report(&mut self, path: &Path, problem: &Problem) {
        self.err(&format!("{}: {problem}", path.display()));
    }

    fn write_file(&mut self, path: &Path, text: &str) -> bool {
        match self.host.write(path, text) {
            Ok(()) => true,
            Err(e) => {
                self.err(&format!("{}: cannot write the file: {e}", path.display()));
                false
            }
        }
    }

    fn read_text(&mut self, path: &Path) -> Result<String, Problem> {
        let cannot = |e: &dyn std::fmt::Display| format!("cannot read the file: {e}");
        let bytes = self
            .host
            .read(path)
            .map_err(|e| Problem::new("", cannot(&e)))?;
        String::from_utf8(bytes).map_err(|_| {
            let e = io::Error::new(
                io::ErrorKind::InvalidData,
                "stream did not contain valid UTF-8",
            );
            Problem::new("", cannot(&e))
        })
    }

    /// Reads a JSON or YAML file into its format, its text and its value.
    fn read(&mut self, path: &Path) -> Result<(Format, String, Value), Problem> {
        let format = data_format_of(path)?;
        let text = self.read_text(path)?;
        let doc = parse(&text, format)?;
        Ok((format, text, doc))
    }

    /// Reads a JSON, YAML or Markdown file into a document. Warnings go to
    /// standard error.
    fn read_any(&mut self, path: &Path) -> Result<Value, Vec<Problem>> {
        self.read_any_with_format(path).map(|(doc, _)| doc)
    }

    /// [`Self::read_any`], with the format of the file. The reader of
    /// Markdown checks the document, so a Markdown document that it gives has
    /// no problems.
    fn read_any_with_format(&mut self, path: &Path) -> Result<(Value, FileFormat), Vec<Problem>> {
        match format_of(path).map_err(|p| vec![p])? {
            FileFormat::Data(format) => {
                let text = self.read_text(path).map_err(|p| vec![p])?;
                let doc = parse(&text, format).map_err(|p| vec![p])?;
                Ok((doc, FileFormat::Data(format)))
            }
            FileFormat::Markdown => {
                let text = self.read_text(path).map_err(|p| vec![p])?;
                let name = path.file_name().and_then(|n| n.to_str());
                let fill = self.host.fill();
                match markdown::read_with(&text, name, &fill) {
                    Ok((doc, warnings)) => {
                        for w in &warnings {
                            self.err(&format!("{}: warning: {w}", path.display()));
                        }
                        Ok((doc, FileFormat::Markdown))
                    }
                    Err(errors) => Err(errors.iter().map(markdown_problem).collect()),
                }
            }
        }
    }

    /// Reads a file, or reports its problems and gives `None`.
    fn read_or_report(&mut self, path: &Path) -> Option<Value> {
        match self.read_any(path) {
            Ok(doc) => Some(doc),
            Err(problems) => {
                for p in &problems {
                    self.report(path, p);
                }
                None
            }
        }
    }

    /// Reads a JSON, YAML or Markdown document and checks it. Reports each
    /// problem.
    fn read_valid(&mut self, path: &Path) -> Option<Value> {
        let doc = self.read_or_report(path)?;
        let problems = check(&doc);
        if problems.is_empty() {
            return Some(doc);
        }
        for p in &problems {
            self.report(path, p);
        }
        self.err(&format!(
            "{}: fix the {} first.",
            path.display(),
            plural(problems.len(), "problem")
        ));
        None
    }

    fn check(&mut self, files: &[PathBuf], json: bool) -> bool {
        let results: Vec<FileProblems> = files
            .iter()
            .map(|path| FileProblems {
                file: path.display().to_string(),
                problems: match self.read_any_with_format(path) {
                    // The Markdown reader already checked the document.
                    Ok((_, FileFormat::Markdown)) => vec![],
                    Ok((doc, FileFormat::Data(_))) => check(&doc),
                    Err(problems) => problems,
                },
            })
            .collect();
        let with_problems = results.iter().filter(|r| !r.problems.is_empty()).count();

        if json {
            let mut text = serde_json::to_string_pretty(&results).expect("problems serialize");
            text.push('\n');
            self.out(&text);
        } else {
            let lines: String = results
                .iter()
                .flat_map(|r| r.problems.iter().map(move |p| format!("{}: {p}\n", r.file)))
                .collect();
            self.out(&lines);
            let count: usize = results.iter().map(|r| r.problems.len()).sum();
            if count == 0 {
                self.err(&format!("{}: no problems.", plural(files.len(), "file")));
            } else {
                self.err(&format!(
                    "{} in {} of {}.",
                    plural(count, "problem"),
                    with_problems,
                    plural(files.len(), "file")
                ));
            }
        }
        with_problems == 0
    }

    fn fmt(&mut self, files: &[PathBuf], check_only: bool) -> bool {
        let mut ok = true;
        for path in files {
            let (format, old_text, doc) = match self.read(path) {
                Ok(r) => r,
                Err(problem) => {
                    self.report(path, &problem);
                    ok = false;
                    continue;
                }
            };
            let text = write(&doc, format);
            if text == old_text {
                continue;
            }
            if check_only {
                self.out(&format!("{}\n", path.display()));
                ok = false;
            } else {
                ok &= self.write_file(path, &text);
            }
        }
        ok
    }

    fn convert(
        &mut self,
        file: &Path,
        to: Target,
        output: Option<&Path>,
        inputs: &[PathBuf],
    ) -> bool {
        let Some(doc) = self.read_or_report(file) else {
            return false;
        };
        let text = match to {
            Target::Json => write(&doc, Format::Json),
            Target::Yaml => write(&doc, Format::Yaml),
            Target::Markdown => {
                let mut others = Vec::with_capacity(inputs.len());
                for path in inputs {
                    others.push(match self.read_or_report(path) {
                        Some(doc) => doc,
                        None => return false,
                    });
                }
                markdown::write(&doc, &others)
            }
        };
        match output {
            Some(path) => self.write_file(path, &text),
            None => self.out(&text),
        }
    }

    fn import(&mut self, file: &Path, stored_path: &Path, output: Option<&Path>) -> bool {
        if format_of(file).ok() != Some(FileFormat::Markdown) {
            self.report(file, &Problem::new("", "the file name must end with .md"));
            return false;
        }
        let (stored_format, _, stored) = match self.read(stored_path) {
            Ok(r) => r,
            Err(problem) => {
                self.report(stored_path, &problem);
                return false;
            }
        };
        let format = match output.map(data_format_of) {
            Some(Ok(format)) => format,
            Some(Err(problem)) => {
                self.report(output.expect("given"), &problem);
                return false;
            }
            None => stored_format,
        };
        let Some(read_doc) = self.read_or_report(file) else {
            return false;
        };
        let now = self.host.fill().now;
        let doc = markdown::next_version_at(&stored, &read_doc, &now);
        let problems = check(&doc);
        if !problems.is_empty() {
            for p in &problems {
                self.report(file, p);
            }
            return false;
        }
        let version = |d: &Value| d.get("version").map(Value::to_string).unwrap_or_default();
        let stored_final = stored.get("status").and_then(Value::as_str) == Some("final");
        let what = if !stored_final {
            format!("the file replaces draft version {}", version(&doc))
        } else if doc == stored {
            format!(
                "no change: version {} is final and its content is the same",
                version(&stored)
            )
        } else {
            format!("new draft version {}", version(&doc))
        };
        self.err(&format!("{}: {what}", file.display()));
        let text = write(&doc, format);
        match output {
            Some(path) => self.write_file(path, &text),
            None => self.out(&text),
        }
    }

    fn ids(&mut self, files: &[PathBuf], check_only: bool) -> bool {
        let mut ok = true;
        for path in files {
            let (format, _, mut doc) = match self.read(path) {
                Ok(r) => r,
                Err(problem) => {
                    self.report(path, &problem);
                    ok = false;
                    continue;
                }
            };
            let report_ids = match assign_ids(&mut doc) {
                Ok(r) => r,
                Err(problems) => {
                    for p in &problems {
                        self.report(path, p);
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
            self.out(&format!("{}: {}\n", path.display(), changes.join(", ")));
            if check_only {
                ok = false;
            } else {
                ok &= self.write_file(path, &write(&doc, format));
            }
        }
        ok
    }

    /// Every file under a folder, as paths from `root` with `/` between
    /// folders, in sorted order. A missing folder has no files.
    fn files_under(&mut self, root: &Path, folder: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut stack = vec![root.join(folder)];
        while let Some(dir) = stack.pop() {
            let Ok(names) = self.host.read_dir(&dir) else {
                continue;
            };
            for name in names {
                let path = dir.join(name);
                if self.host.is_dir(&path) {
                    stack.push(path);
                } else if let Ok(rel) = path.strip_prefix(root) {
                    let parts: Vec<String> = rel
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy().to_string())
                        .collect();
                    out.push(parts.join("/"));
                }
            }
        }
        out.sort();
        out
    }

    /// The paths of `Export::owned`, with each folder (a path that ends with
    /// `/`) replaced by the files under it.
    fn owned_files(&mut self, root: &Path, owned: &[String]) -> Vec<String> {
        let mut out = Vec::new();
        for p in owned {
            match p.strip_suffix('/') {
                Some(folder) => out.extend(self.files_under(root, folder)),
                None => out.push(p.clone()),
            }
        }
        out
    }

    /// The OpenSpec files that an export reads: the main specs and the
    /// Markdown files of the open changes. Archived changes are not read.
    fn openspec_files(&mut self, root: &Path) -> Vec<export::File> {
        let specs = self
            .files_under(root, "openspec/specs")
            .into_iter()
            .filter(|p| p.ends_with("/spec.md"));
        let changes = self
            .files_under(root, "openspec/changes")
            .into_iter()
            .filter(|p| !p.starts_with("openspec/changes/archive/") && p.ends_with(".md"));
        let paths: Vec<String> = specs.chain(changes).collect();
        paths
            .into_iter()
            .filter_map(|path| {
                let bytes = self.host.read(&root.join(&path)).ok()?;
                let text = String::from_utf8(bytes).ok()?;
                Some(export::File { path, text })
            })
            .collect()
    }

    /// The Spec Kit feature number: the number of the folder `NNN-<name>` in
    /// `specs/` if there is one, or one more than the highest number there.
    fn speckit_number(&mut self, root: &Path, name: &str) -> u32 {
        let mut highest = 0;
        let Ok(folders) = self.host.read_dir(&root.join("specs")) else {
            return 1;
        };
        for folder in folders {
            let Some((n, rest)) = speckit_folder_number(&folder) else {
                continue;
            };
            if rest == name {
                return n;
            }
            highest = highest.max(n);
        }
        highest.saturating_add(1)
    }

    fn export(
        &mut self,
        file: &Path,
        tool: Tool,
        root: &Path,
        mut options: Options,
        force: bool,
        dry_run: bool,
    ) -> bool {
        let Some(doc) = self.read_valid(file) else {
            return false;
        };
        if tool == Tool::SpecKit && options.number.is_none() {
            options.number = Some(self.speckit_number(root, &export::name(&doc, &options)));
        }
        if tool == Tool::OpenSpec {
            options.existing = self.openspec_files(root);
        }
        let out = match export::export(&doc, tool, &options) {
            Ok(out) => out,
            Err(e) => {
                self.err(&format!("{}: {e}", file.display()));
                return false;
            }
        };

        // Files of an earlier export that this export does not write. They
        // contain old items, so they must go.
        let stale: Vec<PathBuf> = self
            .owned_files(root, &out.owned)
            .into_iter()
            .filter(|p| !out.files.iter().any(|f| f.path == *p))
            .map(|p| root.join(p))
            .filter(|p| self.host.exists(p))
            .collect();
        let existing: Vec<PathBuf> = out
            .files
            .iter()
            .map(|f| root.join(&f.path))
            .filter(|p| self.host.exists(p))
            .collect();
        if !force && !dry_run && (!existing.is_empty() || !stale.is_empty()) {
            for p in &existing {
                self.err(&format!(
                    "{}: the file exists. Use --force to write over it.",
                    p.display()
                ));
            }
            for p in &stale {
                self.err(&format!(
                    "{}: the file is left over from an earlier export. Use --force to remove it.",
                    p.display()
                ));
            }
            return false;
        }

        let mut ok = true;
        for f in &out.files {
            let path = root.join(&f.path);
            if dry_run {
                self.out(&format!("{}\n", path.display()));
                continue;
            }
            if let Some(parent) = path.parent() {
                if let Err(e) = self.host.create_dir_all(parent) {
                    self.err(&format!(
                        "{}: cannot make the folder: {e}",
                        parent.display()
                    ));
                    ok = false;
                    continue;
                }
            }
            if self.write_file(&path, &f.text) {
                self.out(&format!("{}\n", path.display()));
            } else {
                ok = false;
            }
        }
        for path in &stale {
            if dry_run {
                self.out(&format!("{} (remove)\n", path.display()));
                continue;
            }
            match self.host.remove_file(path) {
                Ok(()) => {
                    self.err(&format!(
                        "removed {}: this export has no content for it",
                        path.display()
                    ));
                    // Remove the folders that are now empty, up to the export
                    // folder. OpenSpec counts any file under specs/.
                    let top = root.join(&out.folder);
                    let mut dir = path.parent();
                    while let Some(d) = dir.filter(|d| *d != top && d.starts_with(&top)) {
                        if self.host.remove_dir(d).is_err() {
                            break;
                        }
                        dir = d.parent();
                    }
                }
                Err(e) => {
                    self.err(&format!("{}: cannot remove the file: {e}", path.display()));
                    ok = false;
                }
            }
        }
        for w in &out.warnings {
            self.err(&format!("warning: {w}"));
        }
        if !out.not_exported.is_empty() {
            self.err(&format!(
                "Not exported to {} ({}):",
                tool.display_name(),
                plural(out.not_exported.len(), "item")
            ));
            for n in &out.not_exported {
                self.err(&format!(
                    "  {} ({}): {}",
                    n.reference, n.item_type, n.reason
                ));
            }
        }
        if tool == Tool::SpecKit && !dry_run && ok {
            // Spec Kit finds the feature to work on in .specify/feature.json,
            // not in the folder name. The tool does not change that file.
            self.err(&format!(
                "To work on this feature with Spec Kit, set \"feature_directory\" to \"{}\" in .specify/feature.json, or set SPECIFY_FEATURE_DIRECTORY.",
                out.folder
            ));
        }
        ok
    }

    fn diff(&mut self, old: &Path, new: &Path, json: bool) -> bool {
        let (Some(a), Some(b)) = (self.read_valid(old), self.read_valid(new)) else {
            return false;
        };
        let d = diff::diff(&a, &b);
        if json {
            let mut text = serde_json::to_string_pretty(&d).expect("a diff serializes");
            text.push('\n');
            self.out(&text)
        } else {
            self.out(&d.to_text())
        }
    }
}
