use winit::event::{ModifiersState, VirtualKeyCode};

pub fn map_special_key(key: VirtualKeyCode, mods: ModifiersState) -> Option<Vec<u8>> {
    let ctrl = mods.ctrl();
    let alt = mods.alt();
    let shift = mods.shift();
    let mut out = match key {
        VirtualKeyCode::Return => Some(vec![b'\r']),
        VirtualKeyCode::Back => Some(vec![0x7f]),
        VirtualKeyCode::Tab => {
            if shift {
                Some(b"\x1b[Z".to_vec())
            } else {
                Some(vec![b'\t'])
            }
        }
        VirtualKeyCode::Escape => Some(vec![0x1b]),
        VirtualKeyCode::Up => Some(csi_mod('A', mods)),
        VirtualKeyCode::Down => Some(csi_mod('B', mods)),
        VirtualKeyCode::Right => Some(csi_mod('C', mods)),
        VirtualKeyCode::Left => Some(csi_mod('D', mods)),
        VirtualKeyCode::Home => Some(b"\x1b[H".to_vec()),
        VirtualKeyCode::End => Some(b"\x1b[F".to_vec()),
        VirtualKeyCode::Delete => Some(b"\x1b[3~".to_vec()),
        VirtualKeyCode::Insert => Some(b"\x1b[2~".to_vec()),
        VirtualKeyCode::PageUp => Some(b"\x1b[5~".to_vec()),
        VirtualKeyCode::PageDown => Some(b"\x1b[6~".to_vec()),
        VirtualKeyCode::F1 => Some(b"\x1bOP".to_vec()),
        VirtualKeyCode::F2 => Some(b"\x1bOQ".to_vec()),
        VirtualKeyCode::F3 => Some(b"\x1bOR".to_vec()),
        VirtualKeyCode::F4 => Some(b"\x1bOS".to_vec()),
        VirtualKeyCode::F5 => Some(b"\x1b[15~".to_vec()),
        VirtualKeyCode::F6 => Some(b"\x1b[17~".to_vec()),
        VirtualKeyCode::F7 => Some(b"\x1b[18~".to_vec()),
        VirtualKeyCode::F8 => Some(b"\x1b[19~".to_vec()),
        VirtualKeyCode::F9 => Some(b"\x1b[20~".to_vec()),
        VirtualKeyCode::F10 => Some(b"\x1b[21~".to_vec()),
        VirtualKeyCode::F11 => Some(b"\x1b[23~".to_vec()),
        VirtualKeyCode::F12 => Some(b"\x1b[24~".to_vec()),
        _ => None,
    };

    if out.is_none() && ctrl {
        if let Some(c) = ctrl_letter(key) {
            out = Some(vec![c]);
        }
    }

    if let Some(mut bytes) = out {
        if alt && !bytes.is_empty() && bytes[0] != 0x1b {
            let mut prefixed = vec![0x1b];
            prefixed.append(&mut bytes);
            return Some(prefixed);
        }
        return Some(bytes);
    }

    None
}

pub fn map_received_char(ch: char, mods: ModifiersState) -> Option<Vec<u8>> {
    if mods.logo() {
        return None;
    }
    if mods.ctrl() {
        return None;
    }
    if matches!(ch, '\n' | '\r' | '\t') {
        return None;
    }
    if ch.is_control() {
        return None;
    }
    let mut buf = [0u8; 4];
    let s = ch.encode_utf8(&mut buf);
    let mut out = Vec::new();
    if mods.alt() {
        out.push(0x1b);
    }
    out.extend_from_slice(s.as_bytes());
    Some(out)
}

fn ctrl_letter(key: VirtualKeyCode) -> Option<u8> {
    use VirtualKeyCode::*;
    let ch = match key {
        A => b'a',
        B => b'b',
        C => b'c',
        D => b'd',
        E => b'e',
        F => b'f',
        G => b'g',
        H => b'h',
        I => b'i',
        J => b'j',
        K => b'k',
        L => b'l',
        M => b'm',
        N => b'n',
        O => b'o',
        P => b'p',
        Q => b'q',
        R => b'r',
        S => b's',
        T => b't',
        U => b'u',
        V => b'v',
        W => b'w',
        X => b'x',
        Y => b'y',
        Z => b'z',
        Space => return Some(0),
        LBracket => return Some(0x1b),
        Backslash => return Some(0x1c),
        RBracket => return Some(0x1d),
        Minus => return Some(0x1f),
        _ => return None,
    };
    Some(ch - b'a' + 1)
}

fn csi_mod(final_char: char, mods: ModifiersState) -> Vec<u8> {
    let mut code = 1u8;
    if mods.shift() {
        code += 1;
    }
    if mods.alt() {
        code += 2;
    }
    if mods.ctrl() {
        code += 4;
    }
    if code == 1 {
        format!("\x1b[{}", final_char).into_bytes()
    } else {
        format!("\x1b[1;{}{}", code, final_char).into_bytes()
    }
}
