use anyhow::Result;
use std::{fs::File, io::Write};

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Paths {
    pub output: String,
    pub source_file: String,
}

impl Paths {
    pub fn write_to_output(&self, output: String, relative_file: &str) -> Result<()> {
        std::fs::create_dir_all(&self.output)?;
        let mut file = File::create(&format!("{}/{relative_file}", self.output))?;
        file.write_all(output.as_bytes())?;
        Ok(())
    }
}
