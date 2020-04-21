#![cfg_attr(not(test), no_std)]

#[macro_use]
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use core::fmt;
use shim::io;

pub struct TableWriter<W: io::Write> {
    writer: W,
    width: usize,
    columns: Vec<String>,
    col_widths: Vec<usize>,
    col_counter: usize,
}

impl<W: io::Write> TableWriter<W> {

    pub fn new(writer: W, width: usize, columns: Vec<String>) -> Self {
        let mut col_widths: Vec<_> = columns.iter().map(|s| s.len()).collect();

        // pad all entries after the first. this puts a minimum one space between headers.
        for i in 1..col_widths.len() {
            col_widths[i] += 1;
        }

        let mut total: usize = col_widths.iter().sum();

        'exit: loop {
            for i in 0..col_widths.len() {
                if total >= width {
                    break 'exit;
                }
                col_widths[i] += 1;
                total += 1;
            }
        }

        Self {
            writer,
            width,
            columns,
            col_widths,
            col_counter: 0,
        }
    }

    pub fn get_writer(&mut self) -> &mut W {
        &mut self.writer
    }

    fn do_newline(&mut self) -> io::Result<()> {
        writeln!(self.writer, "")
    }

    pub fn print_str(&mut self, value: &String) -> io::Result<&mut Self> {
        let col = self.col_widths[self.col_counter];
        self.col_counter = (self.col_counter + 1) % self.col_widths.len();
        write!(self.writer, "{1:>0$}", col, value)?;
        if self.col_counter == 0 {
            self.do_newline()?;
        }
        Ok(self)
    }

    pub fn print<F: fmt::Display>(&mut self, value: F) -> io::Result<&mut Self> {
        self.print_str(&format!("{}", value))
    }

    pub fn print_debug<F: fmt::Debug>(&mut self, value: F) -> io::Result<&mut Self> {
        self.print_str(&format!("{:?}", value))
    }

    pub fn finish(&mut self) -> io::Result<()> {
        if self.col_counter != 0 {
            self.col_counter = 0;
            self.do_newline()?;
        }
        Ok(())
    }

    pub fn print_header(&mut self) -> io::Result<()> {
        self.finish()?;

        for i in 0..self.columns.len() {
            let s = self.columns[i].clone();
            self.print(s)?;
        }

        self.finish()?;
        Ok(())
    }


}






