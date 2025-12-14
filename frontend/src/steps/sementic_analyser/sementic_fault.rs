use models::{define_str_enum, error::SoulError};

define_str_enum!(

    pub enum SementicLevel {
        Warning => "warning",
        Error => "error",
        Debug => "debug",
        Note => "note",
    }
);
pub struct SementicFault {
    meessage: SoulError,
    level: SementicLevel,
}
impl SementicFault {
    pub fn error(err: SoulError) -> Self {
        Self {
            meessage: err,
            level: SementicLevel::Error,
        }
    }

    pub fn consume_soul_error(self) -> SoulError {
        self.meessage
    }
    pub fn get_soul_error(&self) -> &SoulError {
        &self.meessage
    }
    pub fn get_level(&self) -> SementicLevel {
        self.level
    }
}
