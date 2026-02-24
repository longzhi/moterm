/// Encode mouse events as terminal escape sequences.

/// Button values for mouse protocol
pub const BUTTON_LEFT: u8 = 0;
pub const BUTTON_MIDDLE: u8 = 1;
pub const BUTTON_RIGHT: u8 = 2;
pub const BUTTON_RELEASE: u8 = 3;
pub const BUTTON_SCROLL_UP: u8 = 64;
pub const BUTTON_SCROLL_DOWN: u8 = 65;

/// Encode a mouse event in SGR format: CSI < Pb ; Px ; Py M/m
pub fn encode_sgr(button: u8, col: usize, row: usize, pressed: bool) -> Vec<u8> {
    let c = col + 1; // 1-based
    let r = row + 1;
    let suffix = if pressed { 'M' } else { 'm' };
    format!("\x1b[<{};{};{}{}", button, c, r, suffix).into_bytes()
}

/// Encode a mouse event in normal (X10) format: CSI M Cb Cx Cy
pub fn encode_normal(button: u8, col: usize, row: usize) -> Vec<u8> {
    let cb = 32 + button;
    let cx = 32 + (col + 1).min(223) as u8;
    let cy = 32 + (row + 1).min(223) as u8;
    vec![0x1b, b'[', b'M', cb, cx, cy]
}
