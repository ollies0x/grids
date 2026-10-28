//! macOS virtual key codes and how to print a shortcut.

use crate::config::{CMD, CTRL, OPT, SHIFT};

pub const K_D: u32 = 2;
pub const K_F: u32 = 3;
pub const K_G: u32 = 5;
pub const K_C: u32 = 8;
pub const K_E: u32 = 14;
pub const K_R: u32 = 15;
pub const K_T: u32 = 17;
pub const K_EQUAL: u32 = 24;
pub const K_MINUS: u32 = 27;
pub const K_U: u32 = 32;
pub const K_I: u32 = 34;
pub const K_RETURN: u32 = 36;
pub const K_J: u32 = 38;
pub const K_K: u32 = 40;
pub const K_DELETE: u32 = 51;
pub const K_LEFT: u32 = 123;
pub const K_RIGHT: u32 = 124;
pub const K_DOWN: u32 = 125;
pub const K_UP: u32 = 126;

pub fn key_name(code: u32) -> String {
    let n = match code {
        0 => "A", 1 => "S", 2 => "D", 3 => "F", 4 => "H", 5 => "G", 6 => "Z", 7 => "X", 8 => "C",
        9 => "V", 11 => "B", 12 => "Q", 13 => "W", 14 => "E", 15 => "R", 16 => "Y", 17 => "T",
        18 => "1", 19 => "2", 20 => "3", 21 => "4", 22 => "6", 23 => "5", 24 => "=", 25 => "9",
        26 => "7", 27 => "-", 28 => "8", 29 => "0", 30 => "]", 31 => "O", 32 => "U", 33 => "[",
        34 => "I", 35 => "P", 36 => "↩", 37 => "L", 38 => "J", 39 => "'", 40 => "K", 41 => ";",
        42 => "\\", 43 => ",", 44 => "/", 45 => "N", 46 => "M", 47 => ".", 48 => "⇥", 49 => "Space",
        50 => "`", 51 => "⌫", 53 => "⎋", 65 => "Num.", 67 => "Num*", 69 => "Num+", 75 => "Num/",
        76 => "⌤", 78 => "Num-", 81 => "Num=", 82 => "Num0", 83 => "Num1", 84 => "Num2",
        85 => "Num3", 86 => "Num4", 87 => "Num5", 88 => "Num6", 89 => "Num7", 91 => "Num8",
        92 => "Num9", 96 => "F5", 97 => "F6", 98 => "F7", 99 => "F3", 100 => "F8", 101 => "F9",
        103 => "F11", 105 => "F13", 107 => "F14", 109 => "F10", 111 => "F12", 113 => "F15",
        114 => "Help", 115 => "↖", 116 => "⇞", 117 => "⌦", 118 => "F4", 119 => "↘", 120 => "F2",
        121 => "⇟", 122 => "F1", 123 => "←", 124 => "→", 125 => "↓", 126 => "↑",
        _ => return format!("#{code}"),
    };
    n.to_string()
}

/// NSEvent modifier flags -> Carbon hotkey modifiers.
pub fn carbon_mods(m: u64) -> u32 {
    let mut c = 0;
    if m & CMD != 0 {
        c |= 0x100;
    }
    if m & SHIFT != 0 {
        c |= 0x200;
    }
    if m & OPT != 0 {
        c |= 0x800;
    }
    if m & CTRL != 0 {
        c |= 0x1000;
    }
    c
}
