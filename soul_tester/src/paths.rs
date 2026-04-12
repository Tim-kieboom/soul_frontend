use anyhow::Result;
use std::{fs::File, io::Write, path::PathBuf, str::FromStr};

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Paths {
    pub output: String,
    pub log_file: String,
    pub source_folder: String,
    pub entry_file_name: String,
}

impl Paths {
    pub fn to_entry_file_path(&self) -> PathBuf {
        let mut path = PathBuf::from_str(&self.source_folder).expect("error is infallible");

        path.push(&self.entry_file_name);
        path
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
