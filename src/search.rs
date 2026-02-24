/// Simple text search through terminal scrollback + screen.
pub struct SearchState {
    pub active: bool,
    pub query: String,
    pub matches: Vec<SearchMatch>,
    pub current: usize,
}

#[derive(Clone, Debug)]
pub struct SearchMatch {
    pub global_row: usize,
    pub col_start: usize,
    pub col_end: usize,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            active: false,
            query: String::new(),
            matches: Vec::new(),
            current: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.active = !self.active;
        if !self.active {
            self.query.clear();
            self.matches.clear();
            self.current = 0;
        }
    }

    pub fn close(&mut self) {
        self.active = false;
        self.query.clear();
        self.matches.clear();
        self.current = 0;
    }

    pub fn push_char(&mut self, ch: char) {
        self.query.push(ch);
    }

    pub fn pop_char(&mut self) {
        self.query.pop();
    }

    pub fn search(&mut self, term: &crate::terminal::Terminal) {
        self.matches.clear();
        if self.query.is_empty() {
            return;
        }
        let q = self.query.to_lowercase();

        // Search scrollback
        for (i, row) in term.scrollback.iter().enumerate() {
            let text: String = row.cells.iter().map(|c| c.ch).collect();
            let lower = text.to_lowercase();
            let mut start = 0;
            while let Some(pos) = lower[start..].find(&q) {
                let col = start + pos;
                self.matches.push(SearchMatch {
                    global_row: i,
                    col_start: col,
                    col_end: col + self.query.len(),
                });
                start = col + 1;
            }
        }

        // Search screen
        let sb_len = term.scrollback.len();
        for (i, row) in term.screen.iter().enumerate() {
            let text: String = row.cells.iter().map(|c| c.ch).collect();
            let lower = text.to_lowercase();
            let mut start = 0;
            while let Some(pos) = lower[start..].find(&q) {
                let col = start + pos;
                self.matches.push(SearchMatch {
                    global_row: sb_len + i,
                    col_start: col,
                    col_end: col + self.query.len(),
                });
                start = col + 1;
            }
        }

        // Clamp current
        if !self.matches.is_empty() {
            self.current = self.current.min(self.matches.len() - 1);
        } else {
            self.current = 0;
        }
    }

    pub fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.current = (self.current + 1) % self.matches.len();
        }
    }

    pub fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            self.current = if self.current == 0 {
                self.matches.len() - 1
            } else {
                self.current - 1
            };
        }
    }

    pub fn current_match(&self) -> Option<&SearchMatch> {
        self.matches.get(self.current)
    }

    pub fn is_highlighted(&self, global_row: usize, col: usize) -> bool {
        self.matches.iter().any(|m| {
            m.global_row == global_row && col >= m.col_start && col < m.col_end
        })
    }

    pub fn is_current_highlight(&self, global_row: usize, col: usize) -> bool {
        if let Some(m) = self.current_match() {
            m.global_row == global_row && col >= m.col_start && col < m.col_end
        } else {
            false
        }
    }
}
