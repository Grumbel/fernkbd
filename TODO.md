# TODO.md

## Tip

- Base commit of this work line: `d8b6ef0` (Initial checkin — Python fernkbd.py)
- Current tip: `44422ec` (Remove original Python script)
- Branch: master

## Done this session

- Rewrote the Python virtual-keyboard script in Rust using the `evdev` crate
  and `libc` for terminal raw mode.
- Same US-layout character → keycode + shift mapping as the original.
- Added Cargo project (GPL-3.0-or-later), README, LICENSE, AGENTS.md.
- Removed the original `fernkbd.py`.

## Open

- Runtime testing on a machine with `/dev/uinput` (left to the user; do not
  spend time in the sandbox on long tests).
- Optional future: more keycodes / non-US layouts if needed.
