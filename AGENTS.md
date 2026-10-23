# AGENTS.md

## Project

fernkbd — small Rust tool that turns terminal input into virtual keyboard
events via uinput/evdev.

## Standing rules

- Deliverables are git bundles only (see handover instructions in the prompt).
- Author: Ingo Ruhnke <grumbel@gmail.com>
- Trailer on every commit: `Co-authored-by: Grok <grok@x.ai>`
- License: GPL-3.0-or-later, REUSE-compliant SPDX headers.

## Build / test

```bash
cargo build --release
# or
nix build
# Runtime test needs /dev/uinput access; prefer handing over the bundle
# rather than long test runs in the sandbox.
```
