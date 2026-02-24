#![allow(dead_code, clippy::manual_range_patterns)]use std::cmp::{max, min};
use std::collections::VecDeque;

use unicode_width::UnicodeWidthChar;

use crate::color::ColorSpec;

pub const SCROLLBACK_LIMIT: usize = 2000;

#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub fg: ColorSpec,
    pub bg: ColorSpec,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            fg: ColorSpec::DefaultFg,
            bg: ColorSpec::DefaultBg,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Cell {
    pub ch: char,
    pub style: Style,
    pub wide_cont: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            style: Style::default(),
            wide_cont: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Row {
    pub cells: Vec<Cell>,
}

impl Row {
    pub fn new(cols: usize) -> Self {
        Self {
            cells: vec![Cell::default(); cols],
        }
    }

    pub fn clear_range(&mut self, start: usize, end: usize, fill: Cell) {
        let s = min(start, self.cells.len());
        let e = min(end, self.cells.len());
        for c in &mut self.cells[s..e] {
            *c = fill;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pos {
    pub row: usize,
    pub col: usize,
}

#[derive(Clone, Debug)]
pub struct Selection {
    pub anchor: Pos,
    pub focus: Pos,
}

impl Selection {
    pub fn normalized(&self) -> (Pos, Pos) {
        if (self.anchor.row, self.anchor.col) <= (self.focus.row, self.focus.col) {
            (self.anchor, self.focus)
        } else {
            (self.focus, self.anchor)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CursorStyle {
    Block,
    Beam,
    Underline,
}

pub struct Terminal {
    cols: usize,
    rows: usize,
    pub screen: Vec<Row>,
    pub scrollback: VecDeque<Row>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub style: Style,
    pub selection: Option<Selection>,
    pub view_scroll: usize,
    pub title: String,
    pub title_changed: bool,
    pub cursor_style: CursorStyle,
    pub bell: bool,
    /// Mouse tracking mode: 0=off, 1000=normal, 1002=button, 1003=any
    pub mouse_mode: u16,
    /// Mouse encoding: false=normal/utf8, true=SGR (1006)
    pub mouse_sgr: bool,
    /// Bracketed paste mode
    pub bracketed_paste: bool,
    /// Alternate screen buffer active
    pub alt_screen: bool,
    /// Scroll region (top, bottom) — 0-indexed, bottom is exclusive
    pub scroll_top: usize,
    pub scroll_bottom: usize,
    /// Saved cursor position
    saved_cursor_row: usize,
    saved_cursor_col: usize,
    /// Reply buffer for DSR responses
    pub reply_buf: Vec<u8>,
}

impl Terminal {
    pub fn new(cols: usize, rows: usize) -> Self {
        let cols = cols.max(1);
        let rows = rows.max(1);
        Self {
            cols,
            rows,
            screen: (0..rows).map(|_| Row::new(cols)).collect(),
            scrollback: VecDeque::with_capacity(SCROLLBACK_LIMIT),
            cursor_row: 0,
            cursor_col: 0,
            style: Style::default(),
            selection: None,
            view_scroll: 0,
            title: String::new(),
            title_changed: false,
            cursor_style: CursorStyle::Block,
            bell: false,
            mouse_mode: 0,
            mouse_sgr: false,
            bracketed_paste: false,
            alt_screen: false,
            scroll_top: 0,
            scroll_bottom: rows,
            saved_cursor_row: 0,
            saved_cursor_col: 0,
            reply_buf: Vec::new(),
        }
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    fn blank_cell(&self) -> Cell {
        Cell {
            ch: ' ',
            style: self.style,
            wide_cont: false,
        }
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    pub fn start_selection(&mut self, pos: Pos) {
        self.selection = Some(Selection {
            anchor: pos,
            focus: pos,
        });
    }

    pub fn update_selection(&mut self, pos: Pos) {
        if let Some(sel) = &mut self.selection {
            sel.focus = pos;
        }
    }

    pub fn set_view_scroll(&mut self, delta: isize) {
        let max_scroll = self.scrollback.len() as isize;
        let next = (self.view_scroll as isize + delta).clamp(0, max_scroll);
        self.view_scroll = next as usize;
    }

    pub fn reset_view_scroll(&mut self) {
        self.view_scroll = 0;
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        let cols = cols.max(1);
        let rows = rows.max(1);
        if cols == self.cols && rows == self.rows {
            return;
        }

        let mut new_screen: Vec<Row> = (0..rows).map(|_| Row::new(cols)).collect();
        let copy_rows = min(self.rows, rows);
        let copy_cols = min(self.cols, cols);
        for (r, new_row) in new_screen.iter_mut().enumerate().take(copy_rows) {
            for c in 0..copy_cols {
                new_row.cells[c] = self.screen[r].cells[c];
            }
        }
        self.cols = cols;
        self.rows = rows;
        self.screen = new_screen;
        self.cursor_row = min(self.cursor_row, rows - 1);
        self.cursor_col = min(self.cursor_col, cols - 1);
        self.scroll_top = 0;
        self.scroll_bottom = rows;
        self.view_scroll = min(self.view_scroll, self.scrollback.len());
    }

    pub fn line_feed(&mut self) {
        if self.cursor_row + 1 >= self.rows {
            self.scroll_up(1);
        } else {
            self.cursor_row += 1;
        }
    }

    pub fn carriage_return(&mut self) {
        self.cursor_col = 0;
    }

    pub fn backspace(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
    }

    pub fn tab(&mut self) {
        let next = ((self.cursor_col / 8) + 1) * 8;
        self.cursor_col = min(next, self.cols.saturating_sub(1));
    }

    fn scroll_up(&mut self, lines: usize) {
        for _ in 0..lines {
            if let Some(first) = self.screen.first().cloned() {
                if self.scrollback.len() == SCROLLBACK_LIMIT {
                    self.scrollback.pop_front();
                }
                self.scrollback.push_back(first);
            }
            if !self.screen.is_empty() {
                self.screen.remove(0);
                self.screen.push(Row::new(self.cols));
            }
        }
        if self.view_scroll > 0 {
            self.view_scroll = min(self.view_scroll + lines, self.scrollback.len());
        }
    }

    pub fn put_char(&mut self, ch: char) {
        if ch == '\0' || ch == '\u{7f}' {
            return;
        }
        let width = UnicodeWidthChar::width(ch).unwrap_or(1).max(1);
        if self.cursor_col >= self.cols {
            self.cursor_col = 0;
            self.line_feed();
        }
        if width == 2 && self.cursor_col + 1 >= self.cols {
            self.cursor_col = 0;
            self.line_feed();
        }
        if self.cursor_row >= self.rows {
            self.cursor_row = self.rows - 1;
        }
        let row = &mut self.screen[self.cursor_row];
        row.cells[self.cursor_col] = Cell {
            ch,
            style: self.style,
            wide_cont: false,
        };
        if width == 2 {
            row.cells[self.cursor_col + 1] = Cell {
                ch: ' ',
                style: self.style,
                wide_cont: true,
            };
        }
        self.cursor_col += width;
        if self.cursor_col >= self.cols {
            self.cursor_col = self.cols;
        }
    }

    pub fn move_cursor(&mut self, row: usize, col: usize) {
        self.cursor_row = min(row, self.rows.saturating_sub(1));
        self.cursor_col = min(col, self.cols.saturating_sub(1));
    }

    pub fn move_rel(&mut self, dr: isize, dc: isize) {
        let nr = (self.cursor_row as isize + dr).clamp(0, self.rows.saturating_sub(1) as isize);
        let nc = (self.cursor_col as isize + dc).clamp(0, self.cols.saturating_sub(1) as isize);
        self.cursor_row = nr as usize;
        self.cursor_col = nc as usize;
    }

    pub fn erase_in_display(&mut self, mode: usize) {
        let fill = self.blank_cell();
        match mode {
            0 => {
                self.erase_in_line(0);
                for r in self.cursor_row + 1..self.rows {
                    self.screen[r].clear_range(0, self.cols, fill);
                }
            }
            1 => {
                for r in 0..self.cursor_row {
                    self.screen[r].clear_range(0, self.cols, fill);
                }
                self.erase_in_line(1);
            }
            2 | 3 => {
                for r in 0..self.rows {
                    self.screen[r].clear_range(0, self.cols, fill);
                }
                if mode == 3 {
                    self.scrollback.clear();
                }
            }
            _ => {}
        }
    }

    pub fn erase_in_line(&mut self, mode: usize) {
        let fill = self.blank_cell();
        let row = &mut self.screen[self.cursor_row];
        match mode {
            0 => row.clear_range(self.cursor_col, self.cols, fill),
            1 => row.clear_range(0, self.cursor_col + 1, fill),
            2 => row.clear_range(0, self.cols, fill),
            _ => {}
        }
    }

    pub fn visible_start_global_row(&self) -> usize {
        let total = self.scrollback.len() + self.screen.len();
        total.saturating_sub(self.rows + self.view_scroll)
    }

    pub fn total_lines(&self) -> usize {
        self.scrollback.len() + self.screen.len()
    }

    pub fn line_at_global(&self, row: usize) -> Option<&Row> {
        if row < self.scrollback.len() {
            self.scrollback.get(row)
        } else {
            self.screen.get(row - self.scrollback.len())
        }
    }

    pub fn visible_line(&self, view_row: usize) -> Option<&Row> {
        let global = self.visible_start_global_row().saturating_add(view_row);
        self.line_at_global(global)
    }

    pub fn pos_for_view(&self, view_row: usize, col: usize) -> Pos {
        let max_col = self.cols.saturating_sub(1);
        let max_row = self.total_lines().saturating_sub(1);
        let row = min(
            self.visible_start_global_row().saturating_add(view_row),
            max_row,
        );
        Pos {
            row,
            col: min(col, max_col),
        }
    }

    pub fn is_selected(&self, global_row: usize, col: usize) -> bool {
        let Some(sel) = &self.selection else {
            return false;
        };
        let (a, b) = sel.normalized();
        if global_row < a.row || global_row > b.row {
            return false;
        }
        if a.row == b.row {
            return col >= a.col && col <= b.col && global_row == a.row;
        }
        if global_row == a.row {
            return col >= a.col;
        }
        if global_row == b.row {
            return col <= b.col;
        }
        true
    }

    pub fn selection_text(&self) -> Option<String> {
        let sel = self.selection.as_ref()?;
        let (a, b) = sel.normalized();
        let mut out = String::new();
        for row_idx in a.row..=b.row {
            let row = self.line_at_global(row_idx)?;
            let start = if row_idx == a.row { a.col } else { 0 };
            let end = if row_idx == b.row {
                b.col
            } else {
                self.cols.saturating_sub(1)
            };
            let mut line = String::new();
            for col in start..=min(end, self.cols.saturating_sub(1)) {
                let cell = row.cells[col];
                if cell.wide_cont {
                    continue;
                }
                line.push(cell.ch);
            }
            while line.ends_with(' ') {
                line.pop();
            }
            out.push_str(&line);
            if row_idx != b.row {
                out.push('\n');
            }
        }
        Some(out)
    }

    pub fn cursor_global_pos(&self) -> Pos {
        Pos {
            row: self.scrollback.len() + self.cursor_row,
            col: self.cursor_col.min(self.cols.saturating_sub(1)),
        }
    }

    pub fn sgr(&mut self, params: &[i64]) {
        if params.is_empty() {
            self.style = Style::default();
            return;
        }
        let mut i = 0;
        while i < params.len() {
            match params[i] {
                0 => self.style = Style::default(),
                39 => self.style.fg = ColorSpec::DefaultFg,
                49 => self.style.bg = ColorSpec::DefaultBg,
                30..=37 => self.style.fg = ColorSpec::Indexed((params[i] - 30) as u8),
                40..=47 => self.style.bg = ColorSpec::Indexed((params[i] - 40) as u8),
                90..=97 => self.style.fg = ColorSpec::Indexed((params[i] - 90 + 8) as u8),
                100..=107 => self.style.bg = ColorSpec::Indexed((params[i] - 100 + 8) as u8),
                38 | 48 => {
                    let is_fg = params[i] == 38;
                    if i + 1 < params.len() {
                        match params[i + 1] {
                            5 if i + 2 < params.len() => {
                                let c = ColorSpec::Indexed(params[i + 2].clamp(0, 255) as u8);
                                if is_fg {
                                    self.style.fg = c;
                                } else {
                                    self.style.bg = c;
                                }
                                i += 2;
                            }
                            2 if i + 4 < params.len() => {
                                let c = ColorSpec::Rgb(
                                    params[i + 2].clamp(0, 255) as u8,
                                    params[i + 3].clamp(0, 255) as u8,
                                    params[i + 4].clamp(0, 255) as u8,
                                );
                                if is_fg {
                                    self.style.fg = c;
                                } else {
                                    self.style.bg = c;
                                }
                                i += 4;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }

    pub fn scroll_view_page(&mut self, pages: isize) {
        let delta = pages * self.rows as isize;
        self.set_view_scroll(delta);
    }

    pub fn clamp_col(&self, col: usize) -> usize {
        min(col, self.cols.saturating_sub(1))
    }

    pub fn clamp_view_row(&self, row: usize) -> usize {
        min(row, self.rows.saturating_sub(1))
    }

    pub fn append_osc_title(&mut self, title: &str) {
        self.title = title.to_string();
        self.title_changed = true;
        // Title handling intentionally omitted in this minimal build.
    }

    pub fn insert_blank_chars(&mut self, count: usize) {
        let fill = self.blank_cell();
        let row = &mut self.screen[self.cursor_row];
        let count = min(count, self.cols.saturating_sub(self.cursor_col));
        if count == 0 {
            return;
        }
        for c in (self.cursor_col..self.cols).rev() {
            if c >= self.cursor_col + count {
                row.cells[c] = row.cells[c - count];
            } else {
                row.cells[c] = fill;
            }
        }
    }

    pub fn delete_chars(&mut self, count: usize) {
        let fill = self.blank_cell();
        let row = &mut self.screen[self.cursor_row];
        let count = min(count, self.cols.saturating_sub(self.cursor_col));
        if count == 0 {
            return;
        }
        for c in self.cursor_col..self.cols {
            if c + count < self.cols {
                row.cells[c] = row.cells[c + count];
            } else {
                row.cells[c] = fill;
            }
        }
    }

    pub fn set_cursor_col(&mut self, col: usize) {
        self.cursor_col = min(col, self.cols.saturating_sub(1));
    }

    pub fn set_cursor_row(&mut self, row: usize) {
        self.cursor_row = min(row, self.rows.saturating_sub(1));
    }

    pub fn scroll_down_lines(&mut self, lines: usize) {
        let lines = min(lines, self.rows);
        for _ in 0..lines {
            self.screen.pop();
            self.screen.insert(0, Row::new(self.cols));
        }
    }

    pub fn scroll_up_lines(&mut self, lines: usize) {
        self.scroll_up(lines);
    }

    pub fn clear_all(&mut self) {
        let fill = self.blank_cell();
        for r in 0..self.rows {
            self.screen[r].clear_range(0, self.cols, fill);
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    pub fn viewport_contains_cursor(&self) -> bool {
        if self.view_scroll != 0 {
            return false;
        }
        true
    }

    pub fn selection_bounds(&self) -> Option<(Pos, Pos)> {
        self.selection.as_ref().map(|s| s.normalized())
    }

    pub fn clamp_global_row(&self, row: usize) -> usize {
        min(row, self.total_lines().saturating_sub(1))
    }

    pub fn ensure_cursor_visible(&mut self) {
        self.view_scroll = 0;
    }

    pub fn home_cursor(&mut self) {
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    pub fn set_scroll_region(&mut self, top: usize, bottom: usize) {
        let bottom = bottom.min(self.rows);
        if top < bottom {
            self.scroll_top = top;
            self.scroll_bottom = bottom;
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    pub fn save_cursor(&mut self) {
        self.saved_cursor_row = self.cursor_row;
        self.saved_cursor_col = self.cursor_col;
    }

    pub fn restore_cursor(&mut self) {
        self.cursor_row = self.saved_cursor_row.min(self.rows.saturating_sub(1));
        self.cursor_col = self.saved_cursor_col.min(self.cols.saturating_sub(1));
    }

    pub fn erase_chars(&mut self, count: usize) {
        let fill = self.blank_cell();
        if self.cursor_row < self.rows {
            let end = (self.cursor_col + count).min(self.cols);
            self.screen[self.cursor_row].clear_range(self.cursor_col, end, fill);
        }
    }

    pub fn reverse_index(&mut self) {
        if self.cursor_row == self.scroll_top {
            // Scroll down within scroll region
            let bottom = self.scroll_bottom.min(self.rows);
            if bottom > self.scroll_top + 1 {
                self.screen.remove(bottom - 1);
                self.screen.insert(self.scroll_top, Row::new(self.cols));
            }
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
        }
    }

    pub fn next_line(&mut self) {
        self.line_feed();
        self.carriage_return();
    }

    pub fn clear_scrollback(&mut self) {
        self.scrollback.clear();
        self.view_scroll = 0;
    }

    pub fn place_str(&mut self, s: &str) {
        for ch in s.chars() {
            self.put_char(ch);
        }
    }

    pub fn normalized_selection(&self) -> Option<(Pos, Pos)> {
        self.selection.as_ref().map(|s| s.normalized())
    }

    pub fn visible_global_row_for_view(&self, view_row: usize) -> usize {
        self.visible_start_global_row() + self.clamp_view_row(view_row)
    }

    pub fn selection_non_empty(&self) -> bool {
        matches!(self.selection_bounds(), Some((a, b)) if a != b)
    }

    pub fn max_view_scroll(&self) -> usize {
        self.scrollback.len()
    }

    pub fn set_selection_focus_from_view(&mut self, view_row: usize, col: usize) {
        let pos = self.pos_for_view(self.clamp_view_row(view_row), self.clamp_col(col));
        self.update_selection(pos);
    }

    pub fn start_selection_from_view(&mut self, view_row: usize, col: usize) {
        let pos = self.pos_for_view(self.clamp_view_row(view_row), self.clamp_col(col));
        self.start_selection(pos);
    }

    /// Select the word at (view_row, col)
    pub fn select_word_at_view(&mut self, view_row: usize, col: usize) {
        let global_row = self.visible_start_global_row() + view_row;
        if let Some(row) = self.line_at_global(global_row) {
            let cells = &row.cells;
            let col = col.min(cells.len().saturating_sub(1));
            // Find word boundaries (non-whitespace / non-special chars)
            let is_word_char = |c: char| c.is_alphanumeric() || c == '_' || c == '-' || c == '.';
            let ch = cells[col].ch;
            if !is_word_char(ch) {
                // Single char selection for non-word chars
                let pos = Pos { row: global_row, col };
                self.selection = Some(Selection { anchor: pos, focus: pos });
                return;
            }
            let mut start = col;
            while start > 0 && is_word_char(cells[start - 1].ch) {
                start -= 1;
            }
            let mut end = col;
            while end + 1 < cells.len() && is_word_char(cells[end + 1].ch) {
                end += 1;
            }
            self.selection = Some(Selection {
                anchor: Pos { row: global_row, col: start },
                focus: Pos { row: global_row, col: end },
            });
        }
    }

    /// Select entire line at view_row
    pub fn select_line_at_view(&mut self, view_row: usize) {
        let global_row = self.visible_start_global_row() + view_row;
        self.selection = Some(Selection {
            anchor: Pos { row: global_row, col: 0 },
            focus: Pos { row: global_row, col: self.cols.saturating_sub(1) },
        });
    }

    /// Select all content (scrollback + screen)
    pub fn select_all(&mut self) {
        let last_row = self.total_lines().saturating_sub(1);
        self.selection = Some(Selection {
            anchor: Pos { row: 0, col: 0 },
            focus: Pos { row: last_row, col: self.cols.saturating_sub(1) },
        });
    }

    pub fn selection_text_or_empty(&self) -> String {
        self.selection_text().unwrap_or_default()
    }

    pub fn clamp_position(&self, mut pos: Pos) -> Pos {
        pos.row = self.clamp_global_row(pos.row);
        pos.col = self.clamp_col(pos.col);
        pos
    }

    pub fn last_visible_global_row(&self) -> usize {
        self.visible_start_global_row() + self.rows.saturating_sub(1)
    }

    pub fn visible_range(&self) -> (usize, usize) {
        let start = self.visible_start_global_row();
        (start, start + self.rows.saturating_sub(1))
    }

    pub fn scroll_view_to_bottom(&mut self) {
        self.view_scroll = 0;
    }

    pub fn selection_contains_row(&self, row: usize) -> bool {
        if let Some((a, b)) = self.normalized_selection() {
            row >= a.row && row <= b.row
        } else {
            false
        }
    }

    pub fn cursor_screen_pos(&self) -> (usize, usize) {
        (
            self.cursor_row,
            self.cursor_col.min(self.cols.saturating_sub(1)),
        )
    }

    pub fn cursor_visible_view_row(&self) -> Option<usize> {
        if self.view_scroll != 0 {
            return None;
        }
        Some(self.cursor_row)
    }

    pub fn ensure_nonzero_size(&mut self) {
        if self.cols == 0 || self.rows == 0 {
            self.cols = max(self.cols, 1);
            self.rows = max(self.rows, 1);
        }
    }
}
