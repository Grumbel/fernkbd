// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Ingo Ruhnke <grumbel@gmail.com>

//! fernkbd — inject keystrokes from a terminal into a virtual uinput keyboard.
//!
//! Anything typed into the terminal is emitted as global key events.
//! Requires write access to `/dev/uinput` (typically root or the `input` group).

use std::collections::HashMap;
use std::io::{self, Read};
use std::os::unix::io::AsRawFd;

use evdev::{
    uinput::VirtualDevice, AttributeSet, EventType, InputEvent, KeyCode,
};
use libc::{self, c_int, termios, TCSADRAIN};

/// Map a printable character to (keycode, needs_shift).
fn build_char_map() -> HashMap<char, (KeyCode, bool)> {
    let mut m = HashMap::new();

    // Letters
    let letters = [
        ('a', KeyCode::KEY_A),
        ('b', KeyCode::KEY_B),
        ('c', KeyCode::KEY_C),
        ('d', KeyCode::KEY_D),
        ('e', KeyCode::KEY_E),
        ('f', KeyCode::KEY_F),
        ('g', KeyCode::KEY_G),
        ('h', KeyCode::KEY_H),
        ('i', KeyCode::KEY_I),
        ('j', KeyCode::KEY_J),
        ('k', KeyCode::KEY_K),
        ('l', KeyCode::KEY_L),
        ('m', KeyCode::KEY_M),
        ('n', KeyCode::KEY_N),
        ('o', KeyCode::KEY_O),
        ('p', KeyCode::KEY_P),
        ('q', KeyCode::KEY_Q),
        ('r', KeyCode::KEY_R),
        ('s', KeyCode::KEY_S),
        ('t', KeyCode::KEY_T),
        ('u', KeyCode::KEY_U),
        ('v', KeyCode::KEY_V),
        ('w', KeyCode::KEY_W),
        ('x', KeyCode::KEY_X),
        ('y', KeyCode::KEY_Y),
        ('z', KeyCode::KEY_Z),
    ];
    for (ch, code) in letters {
        m.insert(ch, (code, false));
        m.insert(ch.to_ascii_uppercase(), (code, true));
    }

    // Digits and their shift symbols
    let digits = [
        ('1', KeyCode::KEY_1, '!'),
        ('2', KeyCode::KEY_2, '@'),
        ('3', KeyCode::KEY_3, '#'),
        ('4', KeyCode::KEY_4, '$'),
        ('5', KeyCode::KEY_5, '%'),
        ('6', KeyCode::KEY_6, '^'),
        ('7', KeyCode::KEY_7, '&'),
        ('8', KeyCode::KEY_8, '*'),
        ('9', KeyCode::KEY_9, '('),
        ('0', KeyCode::KEY_0, ')'),
    ];
    for (ch, code, shifted) in digits {
        m.insert(ch, (code, false));
        m.insert(shifted, (code, true));
    }

    // Whitespace / control-like
    m.insert(' ', (KeyCode::KEY_SPACE, false));
    m.insert('\n', (KeyCode::KEY_ENTER, false));
    m.insert('\r', (KeyCode::KEY_ENTER, false));
    m.insert('\t', (KeyCode::KEY_TAB, false));
    m.insert('\x7f', (KeyCode::KEY_BACKSPACE, false)); // DEL

    // Punctuation
    let punct = [
        ('-', KeyCode::KEY_MINUS, '_'),
        ('=', KeyCode::KEY_EQUAL, '+'),
        ('[', KeyCode::KEY_LEFTBRACE, '{'),
        (']', KeyCode::KEY_RIGHTBRACE, '}'),
        (';', KeyCode::KEY_SEMICOLON, ':'),
        ('\'', KeyCode::KEY_APOSTROPHE, '"'),
        ('`', KeyCode::KEY_GRAVE, '~'),
        ('\\', KeyCode::KEY_BACKSLASH, '|'),
        (',', KeyCode::KEY_COMMA, '<'),
        ('.', KeyCode::KEY_DOT, '>'),
        ('/', KeyCode::KEY_SLASH, '?'),
    ];
    for (ch, code, shifted) in punct {
        m.insert(ch, (code, false));
        m.insert(shifted, (code, true));
    }

    m
}

/// Map a single-byte control character (0x01–0x1f, excluding DEL) to a keycode.
/// Letters become Ctrl+letter; a few others map to common keys.
fn control_to_keycode(b: u8) -> Option<KeyCode> {
    match b {
        // Ctrl+A … Ctrl+Z
        0x01 => Some(KeyCode::KEY_A),
        0x02 => Some(KeyCode::KEY_B),
        0x03 => Some(KeyCode::KEY_C),
        0x04 => Some(KeyCode::KEY_D),
        0x05 => Some(KeyCode::KEY_E),
        0x06 => Some(KeyCode::KEY_F),
        0x07 => Some(KeyCode::KEY_G),
        0x08 => Some(KeyCode::KEY_H),
        0x09 => Some(KeyCode::KEY_I), // also Tab, but Tab is already in char map
        0x0a => Some(KeyCode::KEY_J), // also Line Feed
        0x0b => Some(KeyCode::KEY_K),
        0x0c => Some(KeyCode::KEY_L),
        0x0d => Some(KeyCode::KEY_M), // also CR
        0x0e => Some(KeyCode::KEY_N),
        0x0f => Some(KeyCode::KEY_O),
        0x10 => Some(KeyCode::KEY_P),
        0x11 => Some(KeyCode::KEY_Q),
        0x12 => Some(KeyCode::KEY_R),
        0x13 => Some(KeyCode::KEY_S),
        0x14 => Some(KeyCode::KEY_T),
        0x15 => Some(KeyCode::KEY_U),
        0x16 => Some(KeyCode::KEY_V),
        0x17 => Some(KeyCode::KEY_W),
        0x18 => Some(KeyCode::KEY_X),
        0x19 => Some(KeyCode::KEY_Y),
        0x1a => Some(KeyCode::KEY_Z),
        // Common non-letter controls
        0x1b => Some(KeyCode::KEY_ESC),          // Ctrl+[ / ESC
        0x1c => Some(KeyCode::KEY_BACKSLASH),    // Ctrl+\
        0x1d => Some(KeyCode::KEY_RIGHTBRACE),   // Ctrl+]
        0x1e => Some(KeyCode::KEY_6),            // Ctrl+^
        0x1f => Some(KeyCode::KEY_MINUS),        // Ctrl+_
        _ => None,
    }
}

fn key_event(code: KeyCode, value: i32) -> InputEvent {
    InputEvent::new(EventType::KEY.0, code.0, value)
}

fn syn_event() -> InputEvent {
    InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0)
}

/// Emit a key press/release, optionally with Shift and/or Ctrl held.
fn emit_key(
    device: &mut VirtualDevice,
    code: KeyCode,
    shift: bool,
    ctrl: bool,
) -> io::Result<()> {
    if ctrl {
        device.emit(&[key_event(KeyCode::KEY_LEFTCTRL, 1), syn_event()])?;
    }
    if shift {
        device.emit(&[key_event(KeyCode::KEY_LEFTSHIFT, 1), syn_event()])?;
    }
    device.emit(&[
        key_event(code, 1),
        syn_event(),
        key_event(code, 0),
        syn_event(),
    ])?;
    if shift {
        device.emit(&[key_event(KeyCode::KEY_LEFTSHIFT, 0), syn_event()])?;
    }
    if ctrl {
        device.emit(&[key_event(KeyCode::KEY_LEFTCTRL, 0), syn_event()])?;
    }
    Ok(())
}

/// Put the terminal into raw mode (equivalent to termios::cfmakeraw + tcsetattr).
unsafe fn set_raw(fd: c_int, old: &mut termios) -> io::Result<()> {
    if libc::tcgetattr(fd, old) != 0 {
        return Err(io::Error::last_os_error());
    }
    let mut raw = *old;
    // cfmakeraw equivalent
    raw.c_iflag &= !(libc::IGNBRK
        | libc::BRKINT
        | libc::PARMRK
        | libc::ISTRIP
        | libc::INLCR
        | libc::IGNCR
        | libc::ICRNL
        | libc::IXON);
    raw.c_oflag &= !libc::OPOST;
    raw.c_lflag &= !(libc::ECHO | libc::ECHONL | libc::ICANON | libc::ISIG | libc::IEXTEN);
    raw.c_cflag &= !(libc::CSIZE | libc::PARENB);
    raw.c_cflag |= libc::CS8;
    raw.c_cc[libc::VMIN] = 1;
    raw.c_cc[libc::VTIME] = 0;
    if libc::tcsetattr(fd, TCSADRAIN, &raw) != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

unsafe fn restore(fd: c_int, old: &termios) {
    let _ = libc::tcsetattr(fd, TCSADRAIN, old);
}

fn main() -> io::Result<()> {
    let char_map = build_char_map();

    // Collect every keycode we may emit (plus modifiers)
    let mut keys = AttributeSet::<KeyCode>::new();
    keys.insert(KeyCode::KEY_LEFTSHIFT);
    keys.insert(KeyCode::KEY_LEFTCTRL);
    keys.insert(KeyCode::KEY_ESC);
    for (code, _) in char_map.values() {
        keys.insert(*code);
    }
    // Ensure control-mapped keys are present even if not already in char_map
    for b in 0x01u8..=0x1f {
        if let Some(code) = control_to_keycode(b) {
            keys.insert(code);
        }
    }

    println!("[*] Initializing virtual keyboard via /dev/uinput...");

    let mut device = VirtualDevice::builder()?
        .name("ssh-virtual-keyboard")
        .with_keys(&keys)?
        .build()?;

    // Optional: show the created node(s)
    if let Ok(nodes) = device.enumerate_dev_nodes_blocking() {
        for path in nodes.flatten() {
            println!("[*] Available as {}", path.display());
        }
    }

    let stdin = io::stdin();
    let fd = stdin.as_raw_fd();
    let mut old: termios = unsafe { std::mem::zeroed() };
    unsafe { set_raw(fd, &mut old)? };

    println!("[*] Virtual keyboard active!");
    println!("[*] Anything you type into this terminal will be injected globally.");
    println!("[*] Ctrl combinations (Ctrl+A … Ctrl+Z, ESC, …) are supported.");
    println!("[*] Press Ctrl+C three times in a row to exit.\n");

    let result = (|| -> io::Result<()> {
        let mut stdin = stdin.lock();
        let mut buf = [0u8; 1];
        let mut consecutive_ctrl_c: u8 = 0;

        loop {
            let n = stdin.read(&mut buf)?;
            if n == 0 {
                break;
            }
            let b = buf[0];

            // Ctrl+C (0x03): emit it, but three consecutive ones exit.
            if b == 0x03 {
                consecutive_ctrl_c = consecutive_ctrl_c.saturating_add(1);
                emit_key(&mut device, KeyCode::KEY_C, false, true)?;
                if consecutive_ctrl_c >= 3 {
                    break;
                }
                continue;
            }
            consecutive_ctrl_c = 0;

            // Prefer the printable / special char map (covers Tab, Enter, Backspace, …)
            let ch = b as char;
            if let Some(&(code, shift)) = char_map.get(&ch) {
                emit_key(&mut device, code, shift, false)?;
                continue;
            }

            // Remaining control characters → Ctrl + corresponding key
            if let Some(code) = control_to_keycode(b) {
                // For ESC we emit plain ESC (no Ctrl); for the rest we hold Ctrl.
                let with_ctrl = b != 0x1b;
                emit_key(&mut device, code, false, with_ctrl)?;
            }
            // Unmapped bytes are silently ignored (same as the original)
        }
        Ok(())
    })();

    // Always restore terminal
    unsafe { restore(fd, &old) };
    println!("\n[*] Virtual keyboard stopped. Terminal restored.");

    result
}
