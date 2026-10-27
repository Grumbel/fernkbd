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

/// Map a single-byte control character (0x01–0x1f) to a keycode.
/// Letters become Ctrl+letter; a few others map to common keys.
fn control_to_keycode(b: u8) -> Option<KeyCode> {
    match b {
        0x01 => Some(KeyCode::KEY_A),
        0x02 => Some(KeyCode::KEY_B),
        0x03 => Some(KeyCode::KEY_C),
        0x04 => Some(KeyCode::KEY_D),
        0x05 => Some(KeyCode::KEY_E),
        0x06 => Some(KeyCode::KEY_F),
        0x07 => Some(KeyCode::KEY_G),
        0x08 => Some(KeyCode::KEY_H),
        0x09 => Some(KeyCode::KEY_I),
        0x0a => Some(KeyCode::KEY_J),
        0x0b => Some(KeyCode::KEY_K),
        0x0c => Some(KeyCode::KEY_L),
        0x0d => Some(KeyCode::KEY_M),
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
        0x1b => Some(KeyCode::KEY_ESC),
        0x1c => Some(KeyCode::KEY_BACKSLASH),
        0x1d => Some(KeyCode::KEY_RIGHTBRACE),
        0x1e => Some(KeyCode::KEY_6),
        0x1f => Some(KeyCode::KEY_MINUS),
        _ => None,
    }
}

/// Known CSI / SS3 sequences → keycode (no modifiers).
/// Matched against the bytes that follow ESC.
fn match_escape_sequence(seq: &[u8]) -> Option<KeyCode> {
    match seq {
        // CSI (ESC [ …)
        b"[A" => Some(KeyCode::KEY_UP),
        b"[B" => Some(KeyCode::KEY_DOWN),
        b"[C" => Some(KeyCode::KEY_RIGHT),
        b"[D" => Some(KeyCode::KEY_LEFT),
        b"[H" | b"[1~" => Some(KeyCode::KEY_HOME),
        b"[F" | b"[4~" => Some(KeyCode::KEY_END),
        b"[2~" => Some(KeyCode::KEY_INSERT),
        b"[3~" => Some(KeyCode::KEY_DELETE),
        b"[5~" => Some(KeyCode::KEY_PAGEUP),
        b"[6~" => Some(KeyCode::KEY_PAGEDOWN),
        // Application cursor mode (SS3 / ESC O …)
        b"OA" => Some(KeyCode::KEY_UP),
        b"OB" => Some(KeyCode::KEY_DOWN),
        b"OC" => Some(KeyCode::KEY_RIGHT),
        b"OD" => Some(KeyCode::KEY_LEFT),
        b"OH" => Some(KeyCode::KEY_HOME),
        b"OF" => Some(KeyCode::KEY_END),
        // Function keys (common xterm / vt100 forms)
        b"OP" | b"[11~" => Some(KeyCode::KEY_F1),
        b"OQ" | b"[12~" => Some(KeyCode::KEY_F2),
        b"OR" | b"[13~" => Some(KeyCode::KEY_F3),
        b"OS" | b"[14~" => Some(KeyCode::KEY_F4),
        b"[15~" => Some(KeyCode::KEY_F5),
        b"[17~" => Some(KeyCode::KEY_F6),
        b"[18~" => Some(KeyCode::KEY_F7),
        b"[19~" => Some(KeyCode::KEY_F8),
        b"[20~" => Some(KeyCode::KEY_F9),
        b"[21~" => Some(KeyCode::KEY_F10),
        b"[23~" => Some(KeyCode::KEY_F11),
        b"[24~" => Some(KeyCode::KEY_F12),
        _ => None,
    }
}

fn key_event(code: KeyCode, value: i32) -> InputEvent {
    InputEvent::new(EventType::KEY.0, code.0, value)
}

fn syn_event() -> InputEvent {
    InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0)
}

/// Emit a key press/release, optionally with Shift, Ctrl and/or Alt held.
fn emit_key(
    device: &mut VirtualDevice,
    code: KeyCode,
    shift: bool,
    ctrl: bool,
    alt: bool,
) -> io::Result<()> {
    if alt {
        device.emit(&[key_event(KeyCode::KEY_LEFTALT, 1), syn_event()])?;
    }
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
    if alt {
        device.emit(&[key_event(KeyCode::KEY_LEFTALT, 0), syn_event()])?;
    }
    Ok(())
}

/// Put the terminal into raw mode (equivalent to termios::cfmakeraw + tcsetattr).
unsafe fn set_raw(fd: c_int, old: &mut termios) -> io::Result<()> {
    if libc::tcgetattr(fd, old) != 0 {
        return Err(io::Error::last_os_error());
    }
    let mut raw = *old;
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

/// Read up to `buf.len()` bytes with a timeout (milliseconds). Returns 0 on timeout.
fn read_timeout(fd: c_int, buf: &mut [u8], timeout_ms: i32) -> io::Result<usize> {
    let mut pfd = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    let ret = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };
    if ret < 0 {
        return Err(io::Error::last_os_error());
    }
    if ret == 0 {
        return Ok(0); // timeout
    }
    // Data available — perform a non-blocking style read of what is ready.
    // Temporarily set O_NONBLOCK so we don't block if the sequence is partial.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    let n = unsafe {
        libc::read(
            fd,
            buf.as_mut_ptr() as *mut libc::c_void,
            buf.len(),
        )
    };
    let _ = unsafe { libc::fcntl(fd, libc::F_SETFL, flags) };
    if n < 0 {
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::WouldBlock {
            return Ok(0);
        }
        return Err(err);
    }
    Ok(n as usize)
}

fn main() -> io::Result<()> {
    let char_map = build_char_map();

    // Collect every keycode we may emit (plus modifiers)
    let mut keys = AttributeSet::<KeyCode>::new();
    keys.insert(KeyCode::KEY_LEFTSHIFT);
    keys.insert(KeyCode::KEY_LEFTCTRL);
    keys.insert(KeyCode::KEY_LEFTALT);
    keys.insert(KeyCode::KEY_ESC);

    for (code, _) in char_map.values() {
        keys.insert(*code);
    }
    for b in 0x01u8..=0x1f {
        if let Some(code) = control_to_keycode(b) {
            keys.insert(code);
        }
    }
    // Navigation / function keys used by escape sequences
    for code in [
        KeyCode::KEY_UP,
        KeyCode::KEY_DOWN,
        KeyCode::KEY_LEFT,
        KeyCode::KEY_RIGHT,
        KeyCode::KEY_HOME,
        KeyCode::KEY_END,
        KeyCode::KEY_INSERT,
        KeyCode::KEY_DELETE,
        KeyCode::KEY_PAGEUP,
        KeyCode::KEY_PAGEDOWN,
        KeyCode::KEY_F1,
        KeyCode::KEY_F2,
        KeyCode::KEY_F3,
        KeyCode::KEY_F4,
        KeyCode::KEY_F5,
        KeyCode::KEY_F6,
        KeyCode::KEY_F7,
        KeyCode::KEY_F8,
        KeyCode::KEY_F9,
        KeyCode::KEY_F10,
        KeyCode::KEY_F11,
        KeyCode::KEY_F12,
    ] {
        keys.insert(code);
    }

    println!("[*] Initializing virtual keyboard via /dev/uinput...");

    let mut device = VirtualDevice::builder()?
        .name("ssh-virtual-keyboard")
        .with_keys(&keys)?
        .build()?;

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
    println!("[*] Ctrl combinations, arrows, Home/End, PageUp/Down, F1–F12 supported.");
    println!("[*] Press Ctrl+C three times in a row to exit.\n");

    let result = (|| -> io::Result<()> {
        let mut stdin = stdin.lock();
        let mut buf = [0u8; 1];
        let mut consecutive_ctrl_c: u8 = 0;
        // Scratch buffer for bytes following ESC
        let mut esc_buf = [0u8; 8];

        loop {
            let n = stdin.read(&mut buf)?;
            if n == 0 {
                break;
            }
            let b = buf[0];

            // Ctrl+C (0x03): emit it, three consecutive ones exit.
            if b == 0x03 {
                consecutive_ctrl_c = consecutive_ctrl_c.saturating_add(1);
                emit_key(&mut device, KeyCode::KEY_C, false, true, false)?;
                if consecutive_ctrl_c >= 3 {
                    break;
                }
                continue;
            }
            consecutive_ctrl_c = 0;

            // ESC — try to gather a multi-byte sequence with a short timeout.
            if b == 0x1b {
                // Wait up to 50 ms for the rest of a CSI/SS3 sequence.
                let got = read_timeout(fd, &mut esc_buf, 50)?;
                if got == 0 {
                    // Lone ESC
                    emit_key(&mut device, KeyCode::KEY_ESC, false, false, false)?;
                    continue;
                }

                let seq = &esc_buf[..got];

                // Full known sequence?
                if let Some(code) = match_escape_sequence(seq) {
                    emit_key(&mut device, code, false, false, false)?;
                    continue;
                }

                // Partial / longer sequences: try the longest known prefixes.
                // (e.g. some terminals send extra modifier params we ignore for now)
                let mut matched = false;
                for len in (1..=got).rev() {
                    if let Some(code) = match_escape_sequence(&seq[..len]) {
                        emit_key(&mut device, code, false, false, false)?;
                        // leftover bytes after the match are ignored for simplicity
                        matched = true;
                        break;
                    }
                }
                if matched {
                    continue;
                }

                // ESC + single printable → treat as Alt+key (common Meta convention)
                if got == 1 {
                    let ch = seq[0] as char;
                    if let Some(&(code, shift)) = char_map.get(&ch) {
                        emit_key(&mut device, code, shift, false, true)?;
                        continue;
                    }
                    if let Some(code) = control_to_keycode(seq[0]) {
                        // ESC + control char → Alt+Ctrl+…
                        let with_ctrl = seq[0] != 0x1b;
                        emit_key(&mut device, code, false, with_ctrl, true)?;
                        continue;
                    }
                }

                // Unknown sequence: emit ESC then re-process the following bytes
                // by falling through one at a time would require a push-back buffer.
                // For now emit plain ESC and drop the rest (uncommon).
                emit_key(&mut device, KeyCode::KEY_ESC, false, false, false)?;
                continue;
            }

            // Prefer the printable / special char map (Tab, Enter, Backspace, …)
            let ch = b as char;
            if let Some(&(code, shift)) = char_map.get(&ch) {
                emit_key(&mut device, code, shift, false, false)?;
                continue;
            }

            // Remaining single-byte control characters → Ctrl + key
            if let Some(code) = control_to_keycode(b) {
                let with_ctrl = true; // ESC already handled above
                emit_key(&mut device, code, false, with_ctrl, false)?;
            }
            // Unmapped bytes are silently ignored
        }
        Ok(())
    })();

    unsafe { restore(fd, &old) };
    println!("\n[*] Virtual keyboard stopped. Terminal restored.");

    result
}
