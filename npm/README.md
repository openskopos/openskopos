# npm packages

This folder has the two npm packages of OpenSkopos:

| Package | Contents |
|---|---|
| [`@openskopos/core`](core/README.md) | The WebAssembly build of `skopos-core`, with TypeScript types. It works in Node and in browsers. |
| [`@openskopos/cli`](cli/README.md) | The `skopos` command line tool for Node. It depends on `@openskopos/core` and has no logic of its own. |

The packages have the version of the Rust crates. When the crate version
changes, change `version` in both `package.json` files and the dependency of
`@openskopos/cli` on `@openskopos/core`. `node npm/check-versions.mjs` makes
sure that the versions are the same.

## Build

You need these tools:

- Rust with the target `wasm32-unknown-unknown` (`rustup target add wasm32-unknown-unknown`).
- Node 20 or later.
- The `wasm-bindgen` CLI of the version of the `wasm-bindgen` crate in
  `Cargo.lock`. If it is not on the `PATH`, the build script downloads it
  from the releases of wasm-bindgen on GitHub into the cargo target folder.

To build the WebAssembly module into `npm/core/dist/`, run this command from
the root of the repository:

```bash
npm/build.sh
```

The script runs `cargo build -p skopos-wasm --target wasm32-unknown-unknown --profile wasm --locked`,
and then `wasm-bindgen --target web --remove-name-section --remove-producers-section`.
The `wasm` profile is in the root `Cargo.toml`. It optimizes for size.

getrandom needs the flag `--cfg getrandom_backend="wasm_js"` for this target.
`.cargo/config.toml` sets it. Cargo ignores that file if the environment
variable `RUSTFLAGS` is set, so in that case the script adds the flag to
`RUSTFLAGS`.

## Test

To run the conformance cases and the other tests through `@openskopos/core`,
run this command after the build:

```bash
npm test --prefix npm/core
```

To compare the npm CLI with the native CLI, pack and install the packages,
build the native tool, and run `npm/test/compare.mjs`:

```bash
mkdir -p packs
npm pack ./npm/core --pack-destination packs
npm pack ./npm/cli --pack-destination packs
cargo build -p skopos --release --locked
(mkdir -p /tmp/skopos-npm && cd /tmp/skopos-npm && npm init --yes > /dev/null && npm install "$OLDPWD"/packs/*.tgz)
node npm/test/compare.mjs target/release/skopos /tmp/skopos-npm/node_modules/.bin/skopos
```

The script runs both tools with the same arguments on copies of
`conformance/`. It compares standard output, standard error, the exit code
and the files that each command writes. CI runs it on Linux and macOS.

## Publish

Build first, then publish `@openskopos/core` before `@openskopos/cli`:

```bash
npm publish ./npm/core --access public
npm publish ./npm/cli --access public
```
