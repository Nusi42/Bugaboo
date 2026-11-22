---
name: wsl-build-distribution
description: "Bugaboo's own WSL distribution for Linux release builds; the default WSL distribution belongs to another project"
metadata:
  type: project
---

The computer's default WSL distribution belongs to another project: never change it or build in it. On 2026-11-21 the lead installed a separate Ubuntu 24.04 distribution for Bugaboo (`wsl --install --name ... --location bugaboo.win1/.work/wsl-build`, no admin needed) with build-essential, musl-tools, Rust and the `x86_64-unknown-linux-musl` target, as root. Its name stays out of Git (SEC-PRIVATE); the release build picks it from `.env.local` once that setting exists.

**Why:** the maintainer asked the lead to install the WSL build tools itself; changing the other project's distribution would be wrong.
**How to apply:** don't delete `win1/.work/wsl-build`; point release builds at this distribution, never the default. Windows Sandbox was switched on by the maintainer on 2026-11-21 (`wsb` works).
