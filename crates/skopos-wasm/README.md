# skopos-wasm

`skopos-wasm` is the WebAssembly build of
[`skopos-core`](../skopos-core/README.md) and of the commands of the
[`skopos`](../skopos/README.md) tool. The npm package
[`@openskopos/core`](../../npm/core/README.md) contains this build. The crate
is not on crates.io.

The crate has a `wasm-bindgen` export for each public function of
`skopos-core`. The inputs and the outputs are strings or plain JavaScript
objects.

The crate also exports `runCli`, which runs the commands of the `skopos`
tool. The commands are in the library of the `skopos` crate. They get access
to the file system and the terminal through the trait `skopos::Host`. The
native tool implements `Host` with the Rust standard library. `runCli` gets
its `Host` as an object from JavaScript. The two tools run the same Rust
code, so their output and exit codes are the same.

On targets other than WebAssembly, the crate is empty, so a native build of
the workspace does not compile its dependencies. To build the
npm packages, see [npm/README.md](../../npm/README.md).
