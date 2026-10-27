# TODO.md

## Tip

- Base commit of this work line: `d8b6ef0` (Initial checkin — Python fernkbd.py)
- Current tip: `af267fc` (Add escape-sequence support for arrows, nav and function keys)
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
- Added multi-byte escape sequence parsing (arrows, Home/End, PageUp/Down,
  Insert/Delete, F1–F12) with a short poll timeout after ESC.
- ESC + printable character is treated as Alt+key.

## Open

- Runtime testing on a machine with `/dev/uinput` (left to the user).
- Optional: richer CSI modifier parsing (Ctrl/Shift/Alt combinations on
  arrows and function keys, e.g. ESC [ 1 ; 5 A for Ctrl+Up).
- Optional: non-US layouts if needed.
