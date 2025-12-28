use soul_ast::{define_str_enum, error::SoulError};

define_str_enum!(

    pub enum SementicLevel {
        Error => "error", 0,
        Warning => "warning", 1,
        Debug => "debug", 2,
        Note => "note", 3,
    }
);
pub struct SementicFault {
    meessage: SoulError,
    level: SementicLevel,
}
impl SementicFault {
    pub const fn error(err: SoulError) -> Self {
        Self {
            meessage: err,
            level: SementicLevel::Error,
        }
    }

    pub fn consume_soul_error(self) -> SoulError {
        self.meessage
    }

    pub const fn get_soul_error(&self) -> &SoulError {
        &self.meessage
    }

    pub const fn get_level(&self) -> SementicLevel {
        self.level
    }

    pub const fn is_fatal(&self, fatal_level: SementicLevel) -> bool {
        fatal_level.precedence() <= self.get_level().precedence()
    }
}
