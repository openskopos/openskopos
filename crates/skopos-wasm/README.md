# skopos-wasm

`skopos-wasm` is the WebAssembly build of
[`skopos-core`](../skopos-core/README.md) and of the commands of the
[`skopos`](../skopos/README.md) tool. The npm package
[`@openskopos/core`](../../npm/core/README.md) contains this build. The crate
is not on crates.io.

The crate has a `wasm-bindgen` export for each public function of
`skopos-core`. The inputs and the outputs are strings or plain JavaScript
objects.

The crate also exports `runCli`, which operates the commands of the `skopos`
tool. The commands are in the library of the `skopos` crate. They get access
to the file system and the terminal through the trait `skopos::Host`. The
native tool implements `Host` with the Rust standard library. `runCli` gets
its `Host` as an object from JavaScript. The two tools use the same Rust code.
Thus they write the same output and have the same exit codes.

On targets other than WebAssembly, the crate is empty. Thus a native build of
the workspace does not compile the dependencies of this crate. To build the
npm packages, see [npm/README.md](../../npm/README.md).
