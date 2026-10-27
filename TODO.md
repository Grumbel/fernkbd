# TODO.md

## Tip

- Base commit of this work line: `d8b6ef0` (Initial checkin — Python fernkbd.py)
- Current tip: `41b4772` (Add CLI flags, VERSION scheme, man page; fix arrow-key input)
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
- Fixed stdout mangling under raw mode; status line shows last injected key.
- Fixed cursor-key mishandling (arrows became Shift+A/B/C/D) by using only
  raw fd reads — no mixing StdinLock buffering with poll/read on the fd.
- Added `--help` / `--version` (-h / -V), top-level `VERSION` file, `build.rs`
  that embeds full development version (`0.1.0-dev.N+gHASH`), man page (with
  LIMITATIONS section), and flake version scheme matching the VERSION rules.

## Open

- Runtime testing on a machine with `/dev/uinput` (left to the user).
- Optional: richer CSI modifier parsing (Ctrl/Shift/Alt on arrows / function keys).
- Optional: non-US layouts if needed.
- Mouse reporting deferred until an X11 (or other) client path exists.
