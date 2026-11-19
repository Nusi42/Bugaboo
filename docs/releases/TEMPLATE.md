# Bugaboo {version}

<!-- BEGIN GENERATED build by `cargo xtask release` from bb-core and Cargo.toml -->
<!-- END GENERATED build -->

## What is new

What changed for the people who use Bugaboo, in plain words, one line each.

## Known issues

What does not work yet, and how to get around it.

## Support period

<!-- BEGIN GENERATED support-period by `cargo xtask release` from bb-core and Cargo.toml -->
<!-- END GENERATED support-period -->

## Install or update

- **New installation:** follow [Install Bugaboo](https://github.com/Nusi42/Bugaboo/blob/main/userDocs/install/README.md).
- **Update:** check the new program with the Bugaboo you have installed
  (`bugaboo update verify <file>`), stop the service, replace the program and
  start it again. It makes a backup and brings the database up to date by
  itself ([Update Bugaboo](https://github.com/Nusi42/Bugaboo/blob/main/userDocs/install/upgrade.md)).
- Windows warns about an unknown publisher until Bugaboo's program is signed
  with a certificate; check the signature first.

## Files

| File | What it is |
|------|------------|
| `bugaboo-{version}-windows-x64.exe` | The program for Windows x64 |
| `bugaboo-{version}-linux-x64` | The program for Linux x64, for every distribution and WSL |
| `bugaboo-{version}.cdx.json`, `bugaboo-{version}.spdx.json` | The parts list in the CycloneDX and SPDX formats |
| `bugaboo-{version}-notices.txt` | The licenses of the parts |
| `bugaboo-{version}.vex.json` | Why known vulnerabilities of parts do not affect Bugaboo |
| `bugaboo-{version}-notes.md` | These notes |
| `SHA256SUMS` | The checksum of each file above |
| `*.minisig` | The signature of each file above and of `SHA256SUMS` |

## Checking the files

<!-- BEGIN GENERATED release-keys by `cargo xtask release` from bb-core and Cargo.toml -->
<!-- END GENERATED release-keys -->
