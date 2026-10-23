import sys
import termios
import tty
from evdev import UInput, ecodes as e

# Comprehensive mapping of characters to Linux evdev keycodes and shift requirements
CHAR_MAP = {
    'a': (e.KEY_A, False), 'A': (e.KEY_A, True),
    'b': (e.KEY_B, False), 'B': (e.KEY_B, True),
    'c': (e.KEY_C, False), 'C': (e.KEY_C, True),
    'd': (e.KEY_D, False), 'D': (e.KEY_D, True),
    'e': (e.KEY_E, False), 'E': (e.KEY_E, True),
    'f': (e.KEY_F, False), 'F': (e.KEY_F, True),
    'g': (e.KEY_G, False), 'G': (e.KEY_G, True),
    'h': (e.KEY_H, False), 'H': (e.KEY_H, True),
    'i': (e.KEY_I, False), 'I': (e.KEY_I, True),
    'j': (e.KEY_J, False), 'J': (e.KEY_J, True),
    'k': (e.KEY_K, False), 'K': (e.KEY_K, True),
    'l': (e.KEY_L, False), 'L': (e.KEY_L, True),
    'm': (e.KEY_M, False), 'M': (e.KEY_M, True),
    'n': (e.KEY_N, False), 'N': (e.KEY_N, True),
    'o': (e.KEY_O, False), 'O': (e.KEY_O, True),
    'p': (e.KEY_P, False), 'P': (e.KEY_P, True),
    'q': (e.KEY_Q, False), 'Q': (e.KEY_Q, True),
    'r': (e.KEY_R, False), 'R': (e.KEY_R, True),
    's': (e.KEY_S, False), 'S': (e.KEY_S, True),
    't': (e.KEY_T, False), 'T': (e.KEY_T, True),
    'u': (e.KEY_U, False), 'U': (e.KEY_U, True),
    'v': (e.KEY_V, False), 'V': (e.KEY_V, True),
    'w': (e.KEY_W, False), 'W': (e.KEY_W, True),
    'x': (e.KEY_X, False), 'X': (e.KEY_X, True),
    'y': (e.KEY_Y, False), 'Y': (e.KEY_Y, True),
    'z': (e.KEY_Z, False), 'Z': (e.KEY_Z, True),
    
    '1': (e.KEY_1, False), '!': (e.KEY_1, True),
    '2': (e.KEY_2, False), '@': (e.KEY_2, True),
    '3': (e.KEY_3, False), '#': (e.KEY_3, True),
    '4': (e.KEY_4, False), '$': (e.KEY_4, True),
    '5': (e.KEY_5, False), '%': (e.KEY_5, True),
    '6': (e.KEY_6, False), '^': (e.KEY_6, True),
    '7': (e.KEY_7, False), '&': (e.KEY_7, True),
    '8': (e.KEY_8, False), '*': (e.KEY_8, True),
    '9': (e.KEY_9, False), '(': (e.KEY_9, True),
    '0': (e.KEY_0, False), ')': (e.KEY_0, True),
    
    ' ': (e.KEY_SPACE, False),
    '\n': (e.KEY_ENTER, False),
    '\r': (e.KEY_ENTER, False),
    '\t': (e.KEY_TAB, False),
    '\x7f': (e.KEY_BACKSPACE, False),
    '-': (e.KEY_MINUS, False), '_': (e.KEY_MINUS, True),
    '=': (e.KEY_EQUAL, False), '+': (e.KEY_EQUAL, True),
    '[': (e.KEY_LEFTBRACE, False), '{': (e.KEY_LEFTBRACE, True),
    ']': (e.KEY_RIGHTBRACE, False), '}': (e.KEY_RIGHTBRACE, True),
    ';': (e.KEY_SEMICOLON, False), ':': (e.KEY_SEMICOLON, True),
    '\'': (e.KEY_APOSTROPHE, False), '"': (e.KEY_APOSTROPHE, True),
    '`': (e.KEY_GRAVE, False), '~': (e.KEY_GRAVE, True),
    '\\': (e.KEY_BACKSLASH, False), '|': (e.KEY_BACKSLASH, True),
    ',': (e.KEY_COMMA, False), '<': (e.KEY_COMMA, True),
    '.': (e.KEY_DOT, False), '>': (e.KEY_DOT, True),
    '/': (e.KEY_SLASH, False), '?': (e.KEY_SLASH, True),
}

def main():
    # Setup capabilities for the virtual device
    cap = {
        e.EV_KEY: list(set(code for code, _ in CHAR_MAP.values())) + [e.KEY_LEFTSHIFT]
    }

    print("[*] Initializing virtual keyboard via /dev/uinput...")
    
    # Create the virtual input device
    with UInput(cap, name="ssh-virtual-keyboard", vendor=0x1234, product=0x5678) as ui:
        fd = sys.stdin.fileno()
        old_settings = termios.tcgetattr(fd)
        
        print("[*] Virtual keyboard active!")
        print("[*] Anything you type into this terminal will be injected globally.")
        print("[*] Press Ctrl+C to exit.\n")
        
        try:
            # Put terminal into raw mode to capture characters instantly without echo/buffering
            tty.setraw(fd)
            while True:
                char = sys.stdin.read(1)
                if not char:
                    break
                
                # Ctrl+C to exit
                if char == '\x03':
                    break
                
                if char in CHAR_MAP:
                    keycode, shift_needed = CHAR_MAP[char]
                    
                    if shift_needed:
                        ui.write(e.EV_KEY, e.KEY_LEFTSHIFT, 1)
                    
                    # Press down and release
                    ui.write(e.EV_KEY, keycode, 1)
                    ui.syn()
                    ui.write(e.EV_KEY, keycode, 0)
                    ui.syn()
                    
                    if shift_needed:
                        ui.write(e.EV_KEY, e.KEY_LEFTSHIFT, 0)
                        ui.syn()
        finally:
            # Always restore terminal settings on exit
            termios.tcsetattr(fd, termios.TCSADRAIN, old_settings)
            print("\n[*] Virtual keyboard stopped. Terminal restored.")

if __name__ == '__main__':
    main()