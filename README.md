# Modern Intel Firmware Tool :sparkles:

This is a new utility to analyze and edit firmware images for Intel platforms.

Based on knowledge from [`me_cleaner`](https://github.com/corna/me_cleaner),
[MEAnalyzer](https://github.com/platomav/meanalyzer) and related research,
`intel_fw` is written from scratch in Rust, allowing for integration with other
projects, including a programmatic API.

## Milestones

- [ ] bootstrap a new, general CLI, `intel_fw`, with an `me` subcommand
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
