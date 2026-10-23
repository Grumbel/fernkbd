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

Press `Ctrl+C` to exit. The terminal is restored on exit.

## Character set

Supports the same US-layout mapping as the original Python script:

- `a`–`z` / `A`–`Z`
- `0`–`9` and shifted symbols `!@#$%^&*()`
- Space, Enter, Tab, Backspace
- Common punctuation: `-_=+[]{};:'",.<>/?`\`~|`

Unmapped characters are ignored.

## License

GPL-3.0-or-later
