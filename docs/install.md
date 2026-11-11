# Install the skopos tool

`skopos` is the command line tool for OpenSkopos documents. It checks,
formats and converts documents. It adds IDs to new items, compares versions,
and exports documents to other tools. Each release has binaries for macOS,
Linux and Windows.

## Homebrew

If you use [Homebrew](https://brew.sh) on macOS or Linux, enter this
command:

```bash
brew install openskopos/tap/skopos
```

To install a new release later, enter `brew upgrade skopos`.

## Install script

On macOS or Linux, enter this command. The script downloads the binary for
your system and puts it in `~/.cargo/bin`:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/openskopos/openskopos/releases/latest/download/skopos-installer.sh | sh
```

On Windows, enter this command in PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/openskopos/openskopos/releases/latest/download/skopos-installer.ps1 | iex"
```

## Cargo

If you have a Rust toolchain, install the tool from crates.io:

```bash
cargo install --locked skopos
```

## npm

The npm package `@openskopos/cli` has the same commands. It operates in Node
20 or later on all platforms. You can use it without an installation:

```bash
npx @openskopos/cli check intent.skopos.json
```

To install it, enter `npm install --global @openskopos/cli`.

## Download a binary

The [releases page](https://github.com/openskopos/openskopos/releases) has an
archive for each platform:

| Platform | Archive |
|---|---|
| macOS, Apple silicon | `skopos-aarch64-apple-darwin.tar.xz` |
| macOS, Intel | `skopos-x86_64-apple-darwin.tar.xz` |
| Linux, x86-64 | `skopos-x86_64-unknown-linux-gnu.tar.xz` |
| Linux, x86-64, static (musl) | `skopos-x86_64-unknown-linux-musl.tar.xz` |
| Linux, ARM64 | `skopos-aarch64-unknown-linux-gnu.tar.xz` |
| Windows, x86-64 | `skopos-x86_64-pc-windows-msvc.zip` |

Each archive has a `.sha256` file with its checksum. The file `sha256.sum`
contains the checksums of all archives. Extract the archive, and put `skopos`
in a folder that is on your `PATH`.

## Make sure that the tool works

Enter this command:

```bash
skopos --version
```

The tool writes its version. For all commands, see
[the skopos command line tool](../crates/skopos/README.md).
