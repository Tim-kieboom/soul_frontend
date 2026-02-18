use anyhow::Result;
use std::{fs::File, io::Write, path::PathBuf};

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Paths {
    pub output: String,
    pub log_file: String,
    pub source_file: String,
}

impl Paths {
    pub fn write_multiple_outputs<const N: usize>(&self, ouputs: [(&str, &str); N]) -> Result<()> {
        for (text, file) in ouputs {
            self.write_to_output(text, file)?;
        }
        Ok(())
    }

    pub fn write_to_output(&self, output: &str, relative_file: &str) -> Result<()> {
        let path = PathBuf::from(format!("{}/{relative_file}", self.output));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = File::create(path)?;
        file.write_all(output.as_bytes())?;
        Ok(())
    }
}
