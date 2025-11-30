# Modern Intel Firmware Tool ✨

This is a new utility to analyze and edit firmware images for [Intel platforms](
docs/platforms.md).

Based on [knowledge](docs/knowledge.md) from other projects, such as
`me_cleaner`, coreboot's `ifdtool`, ME Analyzer and related research,
`intel_fw` is written from scratch in Rust, allowing for integration with other
projects, including a flexible API.

The [architecture and design](docs/architecture.md) is based on experience.

To test this tool, you will need sample [firmware images](docs/images.md).
For convenience, take a look at the [scripts](scripts/) used for development.

## Commands

### `me`

The `me` command lets you print, edit and check the (CS)ME firmware.
The `me clean` command is compatible with `me_cleaner`, with minor differences:

- The `--whitelist` and `--blacklist` flags do not cause deletion of partitions
  when multiple partitions refer to the same range, but at least one of them is
  to be retained. This is considered a bug fix.
- The `--check` flag checks _all_ directory partitions as well as the presence
  of the FTPR. Analysis details are printed unconditionally.
- The `--truncuate` option may result in smaller ME images than `me_cleaner`.

## Development

To run the CLI via `cargo` directly, remember to add arguments after an extra
`--`; i.e., to print the general help, invoke `cargo run --relase -- -h`, or,
for a subcommand, e.g. `cargo run --relase -- me clean -h`.

This tool uses the [`clap` command line argument parser in _derive_ mode](https://docs.rs/clap/latest/clap/_derive/index.html).
To familiarize yourself with Rust and common approaches to CLI tools, take a
look at the [Rust CLI book](https://rust-cli.github.io/book/index.html).

For more understanding, see also any of these additional resources:

- <https://rust-cli-recommendations.sunshowers.io/handling-arguments.html>
- <https://github.com/kyclark/command-line-rust>
- <https://tucson-josh.com/posts/rust-clap-cli/>
- <https://www.rustadventure.dev/introducing-clap/clap-v4/parsing-arguments-with-clap>

## Milestones

- [x] bootstrap a new, general CLI, `intel_fw`, with an `me` subcommand
    - mimic the `me_cleaner` CLI, using similar+same arguments and switches for
      compatibility
- [x] port the logic to Rust, using `me_cleaner`-edited images as test fixtures
    - NOTE: committing the test fixtures would be big and a potential license
      issue; instead, add notes on how to reproduce them, via public vendor
      images and extraction utilities (e.g. from Lenovo)
    - [x] step 1: port core logic to produce the same output as `me_cleaner` for
      Lenovo ThinkPad X230 + X270
    - [x] step 2: full feature parity with `me_cleaner`
- [x] expand the documentation
    - [x] higher-level view on Intel platform boot flows
    - [x] how the Intel data structures work, in prose
    - [x] adding support for more platforms and variants
- [ ] work out a reusable library, i.e., a Rust crate for <https://crates.io/>
    - [x] turn all `unwrap()`s into `Option`/`Result`; add lint rule
    - [ ] add bounds checks
    - [ ] publish the crate
- [ ] sync up; <https://github.com/corna/me_cleaner> has another patch that
  coreboot is missing, doing rework and adding ME Gen 1 support
