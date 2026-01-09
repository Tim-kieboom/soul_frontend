use crate::sementic_level::SementicLevel;

macro_rules! define_const_strs {
    ($($name:ident = $value:expr),*,) => {
        $(
            #[allow(unused)]
            pub const $name: &str = $value;
        )*
    };
}

define_const_strs!(
    RED = "\x1b[31m",
    BLUE = "\x1b[34m",
    CYAN = "\x1b[36m",
    BLACK = "\x1b[30m",
    GREEN = "\x1b[32m",
    WHITE = "\x1b[37m",
    YELLOW = "\x1b[33m",
    DEFAULT = "\x1b[0m",
    MAGENTA = "\x1b[35m",
    BRIGHT_RED = "\x1b[91m",
    BRIGHT_BLUE = "\x1b[94m",
    BRIGHT_CYAN = "\x1b[96m",
    BRIGHT_BLACK = "\x1b[90m",
    BRIGHT_GREEN = "\x1b[92m",
    BRIGHT_WHITE = "\x1b[97m",
    BRIGHT_YELLOW = "\x1b[93m",
    BRIGHT_MAGENTA = "\x1b[95m",
);

pub const fn sementic_level_color(level: &SementicLevel) -> &'static str {
    match level {
        SementicLevel::Error => RED,
        SementicLevel::Warning => YELLOW,
        SementicLevel::Debug => CYAN,
        SementicLevel::Note => BLUE,
    }
}
