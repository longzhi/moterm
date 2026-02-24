use std::collections::{HashMap, VecDeque};
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};

use fontdue::{Font, Metrics};

use crate::color::{resolve_color, Rgb, CURSOR_BG, CURSOR_FG, DEFAULT_BG, SELECTION_BG};
use crate::terminal::Terminal;

#[derive(Clone)]
pub struct GlyphBitmap {
    pub metrics: Metrics,
    pub alpha: Vec<u8>,
}

#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
struct GlyphKey {
    ch: char,
    px: u16,
}

pub struct GlyphCache {
    map: HashMap<GlyphKey, GlyphBitmap>,
    order: VecDeque<GlyphKey>,
    cap: usize,
}

impl GlyphCache {
    pub fn new(cap: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            cap,
        }
    }

    fn touch(&mut self, key: GlyphKey) {
        if let Some(i) = self.order.iter().position(|k| *k == key) {
            self.order.remove(i);
        }
        self.order.push_back(key);
    }

    pub fn get_or_insert(&mut self, font: &Font, ch: char, px: f32) -> &GlyphBitmap {
        let key = GlyphKey {
            ch,
            px: px.round() as u16,
        };
        if self.map.contains_key(&key) {
            self.touch(key);
            return self.map.get(&key).unwrap();
        }
        let (metrics, alpha) = font.rasterize(ch, px);
        let bmp = GlyphBitmap { metrics, alpha };
        if self.map.len() >= self.cap {
            if let Some(old) = self.order.pop_front() {
                self.map.remove(&old);
            }
        }
        self.map.insert(key, bmp);
        self.order.push_back(key);
        self.map.get(&key).unwrap()
    }
}

pub struct FontAtlas {
    pub font: Font,
    pub px: f32,
    pub cell_width: usize,
    pub cell_height: usize,
    pub baseline: i32,
    pub line_gap: usize,
    pub cache: Arc<Mutex<GlyphCache>>,
}

impl FontAtlas {
    pub fn new(font: Font, px: f32) -> Self {
        let m = font.metrics('M', px);
        let h = (m.height as i32 + 4).max(px.ceil() as i32 + 2) as usize;
        let w = (m.advance_width.ceil() as i32 + 1).max((px * 0.55) as i32) as usize;
        Self {
            font,
            px,
            cell_width: w.max(1),
            cell_height: h.max(1),
            baseline: (px.ceil() as i32),
            line_gap: 0,
            cache: Arc::new(Mutex::new(GlyphCache::new(4096))),
        }
    }
}

pub struct PixelCanvas {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
}

impl PixelCanvas {
    pub fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            pixels: Vec::new(),
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        if width == self.width && height == self.height {
            return;
        }
        self.width = width;
        self.height = height;
        self.pixels = vec![DEFAULT_BG.to_u32(); width.saturating_mul(height)];
    }

    pub fn clear(&mut self, color: Rgb) {
        self.pixels.fill(color.to_u32());
    }

    fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: Rgb) {
        let x2 = (x + w).min(self.width);
        let y2 = (y + h).min(self.height);
        let c = color.to_u32();
        for yy in y..y2 {
            let row = yy * self.width;
            for xx in x..x2 {
                self.pixels[row + xx] = c;
            }
        }
    }

    fn blend_pixel(&mut self, x: usize, y: usize, fg: Rgb, alpha: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = y * self.width + x;
        let bg_u = self.pixels[idx];
        let bg = Rgb::new(
            ((bg_u >> 16) & 0xff) as u8,
            ((bg_u >> 8) & 0xff) as u8,
            (bg_u & 0xff) as u8,
        );
        self.pixels[idx] = fg.blend_over(bg, alpha).to_u32();
    }
}

pub struct Renderer {
    pub atlas: FontAtlas,
    pub canvas: PixelCanvas,
    pub padding_x: usize,
    pub padding_y: usize,
}

impl Renderer {
    pub fn new(font: Font, px: f32) -> Self {
        Self {
            atlas: FontAtlas::new(font, px),
            canvas: PixelCanvas::new(),
            padding_x: 4,
            padding_y: 4,
        }
    }

    pub fn adjust_font_size(&mut self, delta: f32) {
        let new_px = (self.atlas.px + delta).clamp(8.0, 72.0);
        self.set_font_size(new_px);
    }

    pub fn set_font_size(&mut self, px: f32) {
        let font = self.atlas.font.clone();
        self.atlas = FontAtlas::new(font, px);
    }

    pub fn grid_size_for_pixels(&self, width: usize, height: usize) -> (usize, usize) {
        let usable_w = width.saturating_sub(self.padding_x * 2);
        let usable_h = height.saturating_sub(self.padding_y * 2);
        let cols = (usable_w / self.atlas.cell_width).max(1);
        let rows = (usable_h / self.atlas.cell_height).max(1);
        (cols, rows)
    }

    pub fn surface_size_for_grid(&self, cols: usize, rows: usize) -> (usize, usize) {
        (
            cols * self.atlas.cell_width + self.padding_x * 2,
            rows * self.atlas.cell_height + self.padding_y * 2,
        )
    }

    pub fn render(&mut self, term: &Terminal, width: usize, height: usize) {
        self.canvas.resize(width.max(1), height.max(1));
        self.canvas.clear(DEFAULT_BG);

        let start_global = term.visible_start_global_row();
        let cursor = if term.view_scroll == 0 {
            Some(term.cursor_screen_pos())
        } else {
            None
        };
        for view_row in 0..term.rows() {
            let global_row = start_global + view_row;
            let Some(row) = term.visible_line(view_row) else {
                continue;
            };
            for col in 0..term.cols() {
                let cell = row.cells[col];
                if cell.wide_cont {
                    continue;
                }
                let mut bg = resolve_color(cell.style.bg);
                let mut fg = resolve_color(cell.style.fg);
                if term.is_selected(global_row, col) {
                    bg = SELECTION_BG;
                }
                if matches!(cursor, Some((cursor_row, cursor_col)) if view_row == cursor_row && col == cursor_col)
                {
                    bg = CURSOR_BG;
                    fg = CURSOR_FG;
                }
                let x = self.padding_x + col * self.atlas.cell_width;
                let y = self.padding_y + view_row * self.atlas.cell_height;
                self.canvas
                    .fill_rect(x, y, self.atlas.cell_width, self.atlas.cell_height, bg);
                if cell.ch != ' ' {
                    self.draw_glyph(cell.ch, fg, x, y);
                }
            }
        }
    }

    fn draw_glyph(&mut self, ch: char, color: Rgb, cell_x: usize, cell_y: usize) {
        let glyph = {
            let mut cache = self.atlas.cache.lock().unwrap();
            cache
                .get_or_insert(&self.atlas.font, ch, self.atlas.px)
                .clone()
        };
        if glyph.metrics.width == 0 || glyph.metrics.height == 0 {
            return;
        }
        let gx = cell_x as i32 + glyph.metrics.xmin.max(0);
        let gy = cell_y as i32
            + (self.atlas.baseline - glyph.metrics.height as i32 - glyph.metrics.ymin);
        for yy in 0..glyph.metrics.height {
            for xx in 0..glyph.metrics.width {
                let a = glyph.alpha[yy * glyph.metrics.width + xx];
                if a == 0 {
                    continue;
                }
                let px = gx + xx as i32;
                let py = gy + yy as i32;
                if px >= 0 && py >= 0 {
                    self.canvas.blend_pixel(px as usize, py as usize, color, a);
                }
            }
        }
    }

    pub fn nonzero_dims(width: u32, height: u32) -> (NonZeroU32, NonZeroU32) {
        let w = NonZeroU32::new(width.max(1)).unwrap();
        let h = NonZeroU32::new(height.max(1)).unwrap();
        (w, h)
    }
}
