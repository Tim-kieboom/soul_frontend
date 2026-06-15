use std::{fmt::Arguments, fs::File, io::{Stdout, Write}};

use anyhow::Result;

pub trait Writer {
    fn push_fmt(&mut self, args: Arguments<'_>) -> Result<()>;
    fn push_str(&mut self, str: &str) -> Result<()>;
    fn push_char(&mut self, ch: char) -> Result<()>;
}

impl Writer for String {
    fn push_fmt(&mut self, args: Arguments<'_>) -> Result<()> {
        use std::fmt::write;

        write(self, args)?;
        Ok(())
    }
    
    fn push_str(&mut self, str: &str) -> Result<()> {
        self.push_str(str);
        Ok(())
    }
    
    fn push_char(&mut self, ch: char) -> Result<()> {
        self.push(ch);
        Ok(())
    }
}

impl Writer for Stdout {
    fn push_fmt(&mut self, args: Arguments<'_>) -> Result<()> {
        self.write_fmt(args)?;
        Ok(())
    }

    fn push_str(&mut self, str: &str) -> Result<()> {
        self.write(str.as_bytes())?;
        Ok(())
    }

    fn push_char(&mut self, ch: char) -> Result<()> {
        self.write_fmt(format_args!("{ch}"))?;
        Ok(())
    }
}

impl Writer for File {
    fn push_fmt(&mut self, args: Arguments<'_>) -> Result<()> {
        self.write_fmt(args)?;
        Ok(())
    }

    fn push_str(&mut self, str: &str) -> Result<()> {
        self.write(str.as_bytes())?;
        Ok(())
    }

    fn push_char(&mut self, ch: char) -> Result<()> {
        self.write_fmt(format_args!("{ch}"))?;
        Ok(())
    }
}