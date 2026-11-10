# skopos-wasm

`skopos-wasm` is the WebAssembly build of
[`skopos-core`](../skopos-core/README.md) and of the commands of the
[`skopos`](../skopos/README.md) tool. The npm package
[`@openskopos/core`](../../npm/core/README.md) holds it. The crate is not
published on crates.io.

The crate has a `wasm-bindgen` export for each public function of
`skopos-core`. Inputs and outputs are strings or plain JavaScript objects.

It also exports `runCli`, which runs the commands of the `skopos` tool. The
commands are in the library of the `skopos` crate, behind the trait
`skopos::Host`. The trait gives the commands the file system and the
terminal. The native tool gives a host that uses the standard library.
`runCli` takes a host object from JavaScript. Both tools run the same Rust
code, so they give the same output and exit codes.

See [npm/README.md](../../npm/README.md) for how to build the packages.
