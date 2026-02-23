#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorSpec {
    DefaultFg,
    DefaultBg,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn to_u32(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    pub fn blend_over(self, bg: Rgb, alpha: u8) -> Rgb {
        let a = alpha as u16;
        let ia = 255u16.saturating_sub(a);
        let blend = |fg: u8, bg: u8| -> u8 { (((fg as u16) * a + (bg as u16) * ia) / 255) as u8 };
        Rgb::new(
            blend(self.r, bg.r),
            blend(self.g, bg.g),
            blend(self.b, bg.b),
        )
    }
}

pub const DEFAULT_FG: Rgb = Rgb {
    r: 0xe6,
    g: 0xe6,
    b: 0xe6,
};
pub const DEFAULT_BG: Rgb = Rgb {
    r: 0x11,
    g: 0x12,
    b: 0x14,
};
pub const SELECTION_BG: Rgb = Rgb {
    r: 0x33,
    g: 0x66,
    b: 0x99,
};
pub const CURSOR_BG: Rgb = Rgb {
    r: 0xf0,
    g: 0xf0,
    b: 0xf0,
};
pub const CURSOR_FG: Rgb = Rgb {
    r: 0x10,
    g: 0x10,
    b: 0x10,
};

pub fn resolve_color(spec: ColorSpec) -> Rgb {
    match spec {
        ColorSpec::DefaultFg => DEFAULT_FG,
        ColorSpec::DefaultBg => DEFAULT_BG,
        ColorSpec::Rgb(r, g, b) => Rgb::new(r, g, b),
        ColorSpec::Indexed(idx) => ansi256(idx),
    }
}

fn ansi256(idx: u8) -> Rgb {
    const BASE16: [Rgb; 16] = [
        Rgb::new(0x00, 0x00, 0x00),
        Rgb::new(0xcd, 0x31, 0x31),
        Rgb::new(0x0d, 0xbc, 0x79),
        Rgb::new(0xe5, 0xe5, 0x10),
        Rgb::new(0x24, 0x72, 0xc8),
        Rgb::new(0xbc, 0x3f, 0xbc),
        Rgb::new(0x11, 0xa8, 0xcd),
        Rgb::new(0xe5, 0xe5, 0xe5),
        Rgb::new(0x66, 0x66, 0x66),
        Rgb::new(0xf1, 0x4c, 0x4c),
        Rgb::new(0x23, 0xd1, 0x8b),
        Rgb::new(0xf5, 0xf5, 0x43),
        Rgb::new(0x3b, 0x8e, 0xff),
        Rgb::new(0xd6, 0x70, 0xd6),
        Rgb::new(0x29, 0xb8, 0xdb),
        Rgb::new(0xff, 0xff, 0xff),
    ];
    match idx {
        0..=15 => BASE16[idx as usize],
        16..=231 => {
            let i = idx - 16;
            let r = i / 36;
            let g = (i % 36) / 6;
            let b = i % 6;
            let cv = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            Rgb::new(cv(r), cv(g), cv(b))
        }
        232..=255 => {
            let c = 8 + (idx - 232) * 10;
            Rgb::new(c, c, c)
        }
    }
}
