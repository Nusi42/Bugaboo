# M1 follow-ups

Points found while building M1 that were accepted for later. Each names where it will be done. Until Bugaboo tracks its own issues, this list is the issue list for M1; the IDs (`M1-F01` …) are what suppressions and TODOs refer to.

| ID | What | Why later | Planned in |
|----|------|-----------|------------|
| M1-F01 | No Windows ARM64 program (`aarch64-pc-windows-msvc`): the MSVC ARM64 build tools are not installed on the development PC, and `ring` needs them to compile its C and assembly code | Cannot be built or tested here; the target is not in `rust-toolchain.toml`, so checks do not pretend to cover it | When the "MSVC ARM64 build tools" component is installed, or on a Windows ARM64 CI runner |
| M1-F02 | The static Linux programs (x64 and ARM64, musl) are built by `cargo xtask ci` on Linux and prepared in the CI workflow, but have never run | Same reason as M0-F02: no Linux machine yet; using the runner's `gcc` for `ring` on the musl targets is untested | When the repository moves to GitHub (Linux CI runners) |
| M1-F03 | A running server renews its generated HTTPS certificate only when it starts (certificates are valid one year and renewed when under 30 days are left) | A server that runs for more than 11 months without a restart would serve an expired certificate; reloading needs a timer in the server | M2 (the status page shows certificate expiry), with ACME in V1 |
| M1-F04 | Linting the Linux code on Windows switches off the C build step of `ring` through a Cargo override named after `ring`'s `links` value | No C compiler for Linux exists on the development PC; the override is derived from `Cargo.lock`, so it follows version changes, but another crate with a C build step would need the same treatment | When another crate with a C build step is added |
| M1-F05 | The server does not yet react to Windows service control messages | The Windows service (`bugaboo service install`) comes with OPS-04 | OPS-04 |
| M1-F06 | No time limit for reading a request body or for a whole request (only for the headers, idle connections and TLS handshakes) | No endpoint reads a body yet | The first endpoint that reads a body (M1 WP-06) |
| M1-F07 | `cargo xtask check staged` exports the index into `target/staged-tree`; Git-based checks (secrets, file sizes) still read the real repository, and Cargo-based checks run in the export without the build cache | Exact for content; only speed differs | When staged checks become slow |
