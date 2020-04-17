
pub mod text;

#[derive(Copy, Clone, Debug)]
pub enum ColorMode {
    RGB,
    BGR,
}

impl ColorMode {
    #[inline(always)]
    pub fn encode(&self, c: Color) -> u32 {
        match self {
            ColorMode::RGB => ((c.2 as u32) << 16) | ((c.1 as u32) << 8) | ((c.0 as u32) << 0),
            ColorMode::BGR => ((c.0 as u32) << 16) | ((c.1 as u32) << 8) | ((c.2 as u32) << 0),
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Color(pub u8, pub u8, pub u8);

impl From<u32> for Color {
    fn from(a: u32) -> Self {
        Color(((a >> 16) & 0xFF) as u8, ((a >> 8) & 0xFF) as u8, ((a >> 0) & 0xFF) as u8)
    }
}

pub struct DisplayConfig {
    pub width: usize,
    pub height: usize,
    pub pitch: usize, // width of a full row in units of u32
    pub mode: ColorMode,
    pub lfb: &'static mut [u32],
}

impl DisplayConfig {
    #[inline(always)]
    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x >= self.width || y >= self.height {
            return;
        }
        let color = self.mode.encode(color);
        self.lfb[y * self.pitch + x] = color;
    }

    #[inline(always)]
    pub fn copy_pixel(&mut self, x_old: usize, y_old: usize, x_new: usize, y_new: usize) {
        if x_new >= self.width || y_new >= self.height {
            return;
        }
        let mut color= 0;
        if x_old < self.width && y_old < self.height {
            color = self.lfb[y_old * self.pitch + x_old];
        }
        self.lfb[y_new * self.pitch + x_new] = color;
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        let lfb = unsafe { core::slice::from_raw_parts_mut(0 as *mut u32, 0) };
        Self { width: 0, height: 0, pitch: 0, mode: ColorMode::RGB, lfb }
    }
}

pub trait DisplayHolder {
    fn with_display<T: FnOnce(&mut DisplayConfig)>(&self, func: T);
}

impl DisplayHolder for &DisplayConfig {
    fn with_display<T: FnOnce(&mut DisplayConfig)>(&self, func: T) {
        func(unsafe { &mut *((*self) as *const DisplayConfig as *mut DisplayConfig) })
    }
}

pub struct Painter<D: DisplayHolder> {
    holder: D,
    background: Option<Color>,
    foreground: Color,
}

impl<D: DisplayHolder> Painter<D> {
    pub fn new(holder: D) -> Self {
        Self { holder, background: Some(0x0.into()), foreground: 0xFFFFFF.into() }
    }

    pub fn set_foreground(&mut self, foreground: Color) {
        self.foreground = foreground;
    }

    pub fn set_background(&mut self, background: Option<Color>) {
        self.background = background;
    }

    pub fn draw_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: Color) {
        for wi in 0..w {
            for hi in 0..h {
                self.holder.with_display(|d| d.set_pixel(x + wi, y + hi, color));
            }
        }
    }

    pub fn draw_strs(&mut self, x: usize, y: usize, text_slices: &[&str]) {
        use font8x8::legacy::BASIC_LEGACY;

        let mut x_offset = 0;
        let mut y_offset = 0;
        for text in text_slices.iter() {
            for char in text.bytes() {
                match char {
                    b'\n' => {
                        x_offset = 0;
                        y_offset += 8;
                    }
                    _ => {
                        if let Some(back) = self.background {
                            self.draw_rect(x + x_offset, y + y_offset, 8, 8, back);
                        }

                        if char as usize >= BASIC_LEGACY.len() {
                            continue;
                        }

                        for (jy, row) in BASIC_LEGACY[char as usize].iter().enumerate() {
                            for jx in 0..8 {
                                if (*row & 1 << jx) != 0 {
                                    self.holder.with_display(|d| d.set_pixel(x + x_offset + jx, y + y_offset + jy, self.foreground));
                                }
                            }
                        }

                        x_offset += 8;
                    }
                }
            }
        }
    }

    pub fn draw_str(&mut self, x: usize, y: usize, text: &str) {
        self.draw_strs(x, y, &[text]);
    }

}



