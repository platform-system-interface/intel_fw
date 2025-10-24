# Modern Intel Firmware Tool :sparkles:

This is a new utility to analyze and edit firmware images for [Intel platforms](
docs/platforms.md).

Based on knowledge from [`me_cleaner`](https://github.com/corna/me_cleaner),
[MEAnalyzer](https://github.com/platomav/meanalyzer) and related research,
`intel_fw` is written from scratch in Rust, allowing for integration with other
projects, including a programmatic API.

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
- [ ] port the logic to Rust, using `me_cleaner`-edited images as test fixtures
    - NOTE: committing the test fixtures would be big and a potential license
      issue; instead, add notes on how to reproduce them, via public vendor
      images and extraction utilities (e.g. from Lenovo)
    - [ ] step 1: port core logic to produce the same output as `me_cleaner` for
        Lenovo ThinkPad X230 + X270
    - [ ] step 2: full parity with `me_cleaner`
- [ ] expand the documentation with a higher-level on Intel platform boot flows
    - [ ] document how the Intel data structures work, in prose
    - [ ] document how to add support for more platforms and variants
- [ ] work out a reusable library, i.e., a Rust crate for <https://crates.io/>
- [ ] sync up; <https://github.com/corna/me_cleaner> has another patch that
      coreboot is missing, doing rework and adding ME Gen 1 support

## Sources of Information

- [`me_cleaner` wiki](https://github.com/corna/me_cleaner/wiki/)

People are regularly [searching for tools and information around Intel platform](
https://community.intel.com/t5/Embedded-Intel-Core-Processors/Where-to-Download-Flash-Image-Tool-fitc/td-p/249920)
firmware, resorting to forums because Intel does not publish what they need or
limits access to the resources to certain customers only, notably not end users.
Here is a short list of places to find useful information.

- [Win-Raid (Level1Techs) Forum](https://winraid.level1techs.com/t/intel-cs-management-engine-drivers-firmware-and-tools-2-15/30719)
- [Badcaps Forum](https://www.badcaps.net/forum/troubleshooting-hardware-devices-and-electronics-theory/troubleshooting-desktop-motherboards-graphics-cards-and-pc-peripherals/105308-fit-csme-tool-flash-image-tool-trusted-download-location)
- [Indiafix Forum](https://www.indiafix.in/2024/09/download-intel-flash-image-tool-fitc.html?m=1)
- [Vinafix Forum](https://vinafix.com/tags/flash-image-tool/)
- [AliSaler](https://www.alisaler.com/intel-me-system-tools-v11-6-r8-flash-image-tool-download/)

### Extraction

Newer tools (e.g., v18) contain Python code that can be extracted with
[pyinstxtractor](https://github.com/extremecoders-re/pyinstxtractor):

```sh
pyinstxtractor.py ../mfit_18.exe
ls -l tools.exe_extracted/plugins/
```
