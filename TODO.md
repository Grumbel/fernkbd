# TODO.md

## Tip

- Base commit of this work line: `d8b6ef0` (Initial checkin — Python fernkbd.py)
- Current tip: `3dd2e35` (Add Ctrl key support and triple-Ctrl-C exit)
- Branch: master

## Done

- Rewrote the Python virtual-keyboard script in Rust using the `evdev` crate
  and `libc` for terminal raw mode.
- Same US-layout character → keycode + shift mapping as the original.
- Added Cargo project (GPL-3.0-or-later), README, LICENSE, AGENTS.md.
- Removed the original `fernkbd.py`.
- Added `flake.nix` + `flake.lock` (nixpkgs + flake-utils, buildRustPackage).
- Added Ctrl support for Ctrl+A…Z and a few other control characters.
- Changed exit sequence to three consecutive Ctrl+C presses so that a
  single Ctrl+C can be injected into the target.

## Open

- Runtime testing on a machine with `/dev/uinput` (left to the user).
- Optional future: more keycodes / non-US layouts if needed.
- Optional future: multi-byte sequences (arrows, function keys, Alt+…).
