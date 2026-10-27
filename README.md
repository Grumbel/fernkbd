# fernkbd

Virtual keyboard that injects terminal input as global key events via Linux
`uinput` / `evdev`.

Anything you type into the terminal is emitted as if it came from a real
keyboard. Useful when working over SSH and you need to type into a local
(or remote desktop) session.

## Requirements

- Linux with `/dev/uinput`
- Permission to write to `/dev/uinput` (root, or membership in the `input` group)

## Build

```bash
cargo build --release
```

## Run

```bash
sudo ./target/release/fernkbd
# or, after adding yourself to the input group:
./target/release/fernkbd
```

```bash
fernkbd --help
fernkbd --version
```

Press **Ctrl+C three times in a row** to exit. The terminal is restored on exit.
A single (or double) Ctrl+C is injected as a real Ctrl+C so the target
application can receive it.

While running, a status line shows the last key that was injected
(e.g. `Ctrl+C`, `Shift+A`, `Up`, `Alt+x`).

## Supported input

### Characters (US layout)

- `a`–`z` / `A`–`Z`
- `0`–`9` and shifted symbols `!@#$%^&*()`
- Space, Enter, Tab, Backspace
- Common punctuation: `-_=+[]{};:'",.<>/?`\`~|`

### Modifiers / control

- Ctrl+A … Ctrl+Z (including Ctrl+C)
- ESC, Ctrl+\, Ctrl+], Ctrl+^, Ctrl+_

### Navigation & function keys (via terminal escape sequences)

- Arrow keys, Home, End, Insert, Delete, PageUp, PageDown
- F1–F12

### Alt (Meta)

- ESC followed by a character is treated as Alt+character (common terminal convention).

Unmapped sequences/bytes are ignored.

## Limitations

Terminal input is a character/escape stream, not hardware key events:

- No standalone modifier holds (Ctrl alone is invisible)
- Tab ≡ Ctrl+I, Enter ≡ Ctrl+M
- Only chords the terminal encodes are available
- CSI sequences with modifier parameters are not fully parsed yet
- US QWERTY mapping only
- No mouse (deferred to an X11-style client path)

See **fernkbd(1)** for details.

## Version

The single source of truth is the top-level `VERSION` file. Development
builds (`*-dev`) embed `revCount` and a short git hash via `build.rs`.

## License

GPL-3.0-or-later

## Nix

```bash
nix build
./result/bin/fernkbd

# or
nix run

# development shell
nix develop
```

A man page is installed at `share/man/man1/fernkbd.1`.
