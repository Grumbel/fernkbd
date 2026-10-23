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

/// Map a character to (keycode, needs_shift).
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

fn key_event(code: KeyCode, value: i32) -> InputEvent {
    InputEvent::new(EventType::KEY.0, code.0, value)
}

fn syn_event() -> InputEvent {
    InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0)
}

fn emit_key(device: &mut VirtualDevice, code: KeyCode, shift: bool) -> io::Result<()> {
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

    // Collect every keycode we may emit (plus LeftShift)
    let mut keys = AttributeSet::<KeyCode>::new();
    keys.insert(KeyCode::KEY_LEFTSHIFT);
    for (code, _) in char_map.values() {
        keys.insert(*code);
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
    println!("[*] Press Ctrl+C to exit.\n");

    let result = (|| -> io::Result<()> {
        let mut stdin = stdin.lock();
        let mut buf = [0u8; 1];
        loop {
            let n = stdin.read(&mut buf)?;
            if n == 0 {
                break;
            }
            let b = buf[0];

            // Ctrl+C
            if b == 0x03 {
                break;
            }

            let ch = b as char;
            if let Some(&(code, shift)) = char_map.get(&ch) {
                emit_key(&mut device, code, shift)?;
            }
            // Unmapped bytes are silently ignored (same as the Python original)
        }
        Ok(())
    })();

    // Always restore terminal
    unsafe { restore(fd, &old) };
    println!("\n[*] Virtual keyboard stopped. Terminal restored.");

    result
}
