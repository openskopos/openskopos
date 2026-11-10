//! The `skopos` command line tool for OpenSkopos documents.
//!
//! The commands are in the library of this crate. This file gives them the
//! file system and the terminal of the standard library.

use std::fs;
use std::io::{self, Write as _};
use std::path::Path;
use std::process::ExitCode;

use anstream::{AutoStream, ColorChoice};
use skopos::{Host, Stream};
use skopos_core::markdown::Fill;

struct StdHost;

impl Host for StdHost {
    fn read(&mut self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn write(&mut self, path: &Path, text: &str) -> io::Result<()> {
        fs::write(path, text)
    }

    fn exists(&mut self, path: &Path) -> bool {
        path.exists()
    }

    fn is_dir(&mut self, path: &Path) -> bool {
        path.is_dir()
    }

    fn read_dir(&mut self, path: &Path) -> io::Result<Vec<String>> {
        Ok(fs::read_dir(path)?
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect())
    }

    fn create_dir_all(&mut self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path)
    }

    fn remove_file(&mut self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn remove_dir(&mut self, path: &Path) -> io::Result<()> {
        fs::remove_dir(path)
    }

    fn stdout(&mut self, text: &str) -> io::Result<()> {
        io::stdout().lock().write_all(text.as_bytes())
    }

    fn stderr(&mut self, text: &str) {
        let _ = io::stderr().lock().write_all(text.as_bytes());
    }

    fn color(&mut self, stream: Stream) -> bool {
        let choice = match stream {
            Stream::Stdout => AutoStream::choice(&io::stdout()),
            Stream::Stderr => AutoStream::choice(&io::stderr()),
        };
        choice != ColorChoice::Never
    }

    fn fill(&mut self) -> Fill {
        Fill::new()
    }
}

fn main() -> ExitCode {
    ExitCode::from(skopos::run(std::env::args_os(), &mut StdHost))
}
