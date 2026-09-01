use std::{
    fmt::Arguments,
    fs::File,
    io::{Stdout, Write},
};

#[macro_export]
macro_rules! push_fmt {
    ($dst:expr, $($arg:tt)*) => {
        $dst.push_fmt(format_args!($($arg)*))
    };
    ($($arg:tt)*) => {
        compile_error!("requires a destination and format arguments, like `write!(dest, \"format string\", args...)`")
    };
}

pub trait Writer {
    type Error: std::error::Error + Send + Sync + 'static;
    fn push_fmt(&mut self, args: Arguments<'_>) -> Result<(), Self::Error>;
    fn writer_flush(&mut self) -> Result<(), Self::Error>;

    fn push_str(&mut self, str: &str) -> Result<(), Self::Error> {
        self.push_fmt(format_args!("{str}"))
    }
    fn push_char(&mut self, ch: char) -> Result<(), Self::Error> {
        self.push_fmt(format_args!("{ch}"))
    }
}

impl Writer for String {
    type Error = std::fmt::Error;

    fn push_fmt(&mut self, args: Arguments<'_>) -> Result<(), Self::Error> {
        std::fmt::write(self, args)?;
        Ok(())
    }

    fn push_str(&mut self, str: &str) -> Result<(), Self::Error> {
        self.push_str(str);
        Ok(())
    }

    fn push_char(&mut self, ch: char) -> Result<(), Self::Error> {
        self.push(ch);
        Ok(())
    }

    fn writer_flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Writer for Stdout {
    type Error = std::io::Error;

    fn push_fmt(&mut self, args: Arguments<'_>) -> Result<(), Self::Error> {
        self.write_fmt(args)?;
        Ok(())
    }

    fn writer_flush(&mut self) -> Result<(), Self::Error> {
        self.flush()?;
        Ok(())
    }
}

impl Writer for File {
    type Error = std::io::Error;

    fn push_fmt(&mut self, args: Arguments<'_>) -> Result<(), Self::Error> {
        self.write_fmt(args)?;
        Ok(())
    }

    fn writer_flush(&mut self) -> Result<(), Self::Error> {
        self.flush()?;
        Ok(())
    }
}
