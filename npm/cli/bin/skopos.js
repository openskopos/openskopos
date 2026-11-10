#!/usr/bin/env node
// The skopos command line tool. The commands are the commands of the native
// tool, built to WebAssembly in @openskopos/core. This file only gives them
// the file system and the terminal of Node.
import fs from "node:fs";
import { runCli } from "@openskopos/core";

// A closed pipe, for example `skopos convert ... | head`, is not an error.
for (const stream of [process.stdout, process.stderr]) {
  stream.on("error", (e) => {
    if (e.code !== "EPIPE") throw e;
  });
}

const host = {
  readFile: (path) => fs.readFileSync(path),
  writeFile: (path, text) => fs.writeFileSync(path, text),
  exists: (path) => fs.existsSync(path),
  isDir: (path) =>
    fs.statSync(path, { throwIfNoEntry: false })?.isDirectory() ?? false,
  readDir: (path) => fs.readdirSync(path),
  createDirAll: (path) => {
    fs.mkdirSync(path, { recursive: true });
  },
  removeFile: (path) => fs.unlinkSync(path),
  removeDir: (path) => fs.rmdirSync(path),
  stdout: (text) => {
    process.stdout.write(text);
  },
  stderr: (text) => {
    process.stderr.write(text);
  },
  color: (stderr) =>
    (stderr ? process.stderr : process.stdout).hasColors?.() ?? false,
};

process.exitCode = runCli(process.argv.slice(2), host);
