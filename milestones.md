# Milestones

## Initial work

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
- [x] work out a reusable library, i.e., a Rust crate for <https://crates.io/>
    - [x] turn all `unwrap()`s into `Option`/`Result`; add lint rule
    - [x] add bounds checks
    - [x] publish the crate
