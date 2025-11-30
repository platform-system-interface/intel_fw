# `intel_fw`

This is both a command-line interface (CLI) and a library for
[analyzing](./analysis.md) and editing [firmware images](images.md) for
[Intel platforms](./platforms.md).

The [architecture](./architecture.md) is based on [knowledge](./knowledge.md)
from prior research.

## CLI

The CLI is made with the [`clap` command line argument parser in _derive_ mode](https://docs.rs/clap/latest/clap/_derive/index.html).
To familiarize yourself with Rust and common approaches to CLI tools, take a
look at the [Rust CLI book](https://rust-cli.github.io/book/index.html).

For more understanding, see also any of these additional resources:

- <https://rust-cli-recommendations.sunshowers.io/handling-arguments.html>
- <https://github.com/kyclark/command-line-rust>
- <https://tucson-josh.com/posts/rust-clap-cli/>
- <https://www.rustadventure.dev/introducing-clap/clap-v4/parsing-arguments-with-clap>
