use vte::{Params, Perform};

use crate::terminal::Terminal;

pub struct VteHandler<'a> {
    pub term: &'a mut Terminal,
}

impl<'a> VteHandler<'a> {
    pub fn new(term: &'a mut Terminal) -> Self {
        Self { term }
    }

    fn params_to_vec(params: &Params) -> Vec<i64> {
        let mut out = Vec::new();
        for p in params.iter() {
            if p.is_empty() {
                out.push(0);
            } else {
                out.push(p[0] as i64);
            }
        }
        out
    }

    fn first_or(params: &[i64], default: usize) -> usize {
        let v = params.first().copied().unwrap_or(default as i64);
        if v <= 0 {
            default
        } else {
            v as usize
        }
    }
}

impl Perform for VteHandler<'_> {
    fn print(&mut self, c: char) {
        self.term.put_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => self.term.line_feed(),
            b'\r' => self.term.carriage_return(),
            8 => self.term.backspace(),
            b'\t' => self.term.tab(),
            0x07 => self.term.bell = true,  // BEL
            0x0c => self.term.clear_all(),
            _ => {}
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params.len() >= 2 {
            if let Ok(cmd) = std::str::from_utf8(params[0]) {
                if matches!(cmd, "0" | "2") {
                    if let Ok(title) = std::str::from_utf8(params[1]) {
                        self.term.append_osc_title(title);
                    }
                }
            }
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let p = Self::params_to_vec(params);

        // DECSET: CSI ? Pm h (enable private modes)
        if action == 'h' && intermediates == [b'?'] {
            for &mode in &p {
                match mode {
                    1000 | 1002 | 1003 => self.term.mouse_mode = mode as u16,
                    1006 => self.term.mouse_sgr = true,
                    2004 => self.term.bracketed_paste = true,
                    1049 | 47 | 1047 => self.term.alt_screen = true,
                    _ => {}
                }
            }
            return;
        }

        // DECRST: CSI ? Pm l (disable private modes)
        if action == 'l' && intermediates == [b'?'] {
            for &mode in &p {
                match mode {
                    1000 | 1002 | 1003 => {
                        if self.term.mouse_mode == mode as u16 {
                            self.term.mouse_mode = 0;
                        }
                    }
                    1006 => self.term.mouse_sgr = false,
                    2004 => self.term.bracketed_paste = false,
                    1049 | 47 | 1047 => self.term.alt_screen = false,
                    _ => {}
                }
            }
            return;
        }

        // DECSCUSR: cursor style (CSI Ps SP q)
        if action == 'q' && intermediates == [b' '] {
            let style = p.first().copied().unwrap_or(0);
            self.term.cursor_style = match style {
                0..=2 => crate::terminal::CursorStyle::Block,
                3..=4 => crate::terminal::CursorStyle::Underline,
                5..=6 => crate::terminal::CursorStyle::Beam,
                _ => crate::terminal::CursorStyle::Block,
            };
            return;
        }

        match action {
            'A' => self.term.move_rel(-(Self::first_or(&p, 1) as isize), 0),
            'B' => self.term.move_rel(Self::first_or(&p, 1) as isize, 0),
            'C' => self.term.move_rel(0, Self::first_or(&p, 1) as isize),
            'D' => self.term.move_rel(0, -(Self::first_or(&p, 1) as isize)),
            'E' => {
                let n = Self::first_or(&p, 1);
                self.term.move_rel(n as isize, 0);
                self.term.carriage_return();
            }
            'F' => {
                let n = Self::first_or(&p, 1);
                self.term.move_rel(-(n as isize), 0);
                self.term.carriage_return();
            }
            'G' => self
                .term
                .set_cursor_col(Self::first_or(&p, 1).saturating_sub(1)),
            'H' | 'f' => {
                let row = p.first().copied().unwrap_or(1).max(1) as usize - 1;
                let col = p.get(1).copied().unwrap_or(1).max(1) as usize - 1;
                self.term.move_cursor(row, col);
            }
            'J' => self
                .term
                .erase_in_display(p.first().copied().unwrap_or(0) as usize),
            'K' => self
                .term
                .erase_in_line(p.first().copied().unwrap_or(0) as usize),
            'L' => self.term.scroll_down_lines(Self::first_or(&p, 1)),
            'M' => self.term.scroll_up_lines(Self::first_or(&p, 1)),
            '@' => self.term.insert_blank_chars(Self::first_or(&p, 1)),
            'P' => self.term.delete_chars(Self::first_or(&p, 1)),
            'S' => self.term.scroll_up_lines(Self::first_or(&p, 1)),
            'T' => self.term.scroll_down_lines(Self::first_or(&p, 1)),
            'd' => self
                .term
                .set_cursor_row(Self::first_or(&p, 1).saturating_sub(1)),
            'm' => self.term.sgr(&p),
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, byte: u8) {
        match byte {
            b'D' => self.term.line_feed(),
            b'E' => self.term.next_line(),
            b'M' => self.term.reverse_index(),
            b'c' => {
                self.term.clear_all();
                self.term.clear_scrollback();
                self.term.home_cursor();
            }
            _ => {}
        }
    }
}
