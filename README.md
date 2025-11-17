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

For inspiration of how to explore and add new features, take a look at the notes
on the [analysis process](docs/analysis.md) and [extending](docs/extend.md).

### Caveats

Be careful: Always check claimed offsets and sizes in binary data structures to
be both reasonable and within the bounds of the given slices. Sizes can be
direct values, such as "this here points to that 8KB other thing", or counts,
which may lead to overallocation. Say a thing that typically has about a dozen
entries uses a `u32` to represent its count, which may end up being huge. As a
defensive measure, implement a maximum, say 100, to avoid running out of memory.

## TODOs

- [ ] sync up; <https://github.com/corna/me_cleaner> has another patch that
  coreboot is missing, doing rework and adding ME Gen 1 support
- [ ] [IFWI](https://github.com/platform-system-interface/intel_fw/issues/80)
  format support

## Funding

The [initial work](milestones.md#initial_work) has been sponsored through the
first [Open Call by the Open Source Firmware Foundation](https://www.osfw.foundation/funding/small-scale-high-impact-firmware-contributions-2025/). We highly appreciate
their support that made a first release of this project possible.

![Open Source Firmware Foundation Logo](docs/osff_logo_400.png)
