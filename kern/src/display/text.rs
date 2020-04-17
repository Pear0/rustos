use shim::io;

use crate::display::{DisplayHolder, Painter};

pub struct TextPainter<D: DisplayHolder> {
    painter: Painter<D>,
    row: usize,
    column: usize,

    max_row: usize,
    max_column: usize,
}

impl<D: DisplayHolder> TextPainter<D> {
    pub fn new(painter: Painter<D>, max_row: usize, max_column: usize) -> Self {
        Self { painter, row: 0, column: 0, max_row, max_column }
    }

    pub fn set_pos(&mut self, row: usize, column: usize) {
        self.row = core::cmp::min(row, self.max_row);
        self.column = core::cmp::min(column, self.max_column);
    }

    pub fn reset(&mut self) {
        self.set_pos(0, 0);
    }

    fn scroll_up(&mut self) {
        for y in 0..8*(self.row-1) {
            let src = y + 8;
            for x in 0..8*self.max_column {
                self.painter.holder.with_display(|d| d.copy_pixel(x, src, x, y));
            }
        }
    }

    pub fn write_char(&mut self, char: u8) {
        match char {
            b'\n' => {
                self.row += 1;
                self.column = 0;
            }
            _ => {
                self.painter.draw_str(self.column * 8, self.row * 8,
                                      unsafe { core::str::from_utf8_unchecked(core::slice::from_ref(&char)) });
                self.column += 1;
                if self.column >= self.max_column {
                    self.column = 0;
                    self.row += 1;
                }

                if self.row >= self.max_row {
                    self.scroll_up();
                    self.row = self.max_row - 1;
                }
            }
        }
    }

}

impl<D: DisplayHolder> io::Write for TextPainter<D> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for char in buf.iter() {
            self.write_char(*char);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

