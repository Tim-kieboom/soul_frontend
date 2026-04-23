use anyhow::Result;
use std::path::PathBuf;
use std::fs::File;
use std::io::Write;

use soul_utils::{SoulToml, CrateStore, IdAlloc, ModuleId};

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Paths {
    pub output: String,
    pub log_file: String,
    pub source_folder: String,
    pub manifest: String,
    pub entry_file_name: Option<String>,
}

impl Paths {
    pub fn to_source_path(&self) -> PathBuf {
        PathBuf::from(&self.source_folder)
    }

    pub fn to_entry_file_path(&self) -> PathBuf {
        let mut path = self.to_source_path();
        path.push(self.entry_file_name.as_deref().unwrap_or("main.soul"));
        path
    }

    pub fn manifest_path(&self) -> PathBuf {
        PathBuf::from(&self.manifest)
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

    pub fn load_crates(&self) -> Result<(SoulToml, CrateStore)> {
        let manifest_path = self.manifest_path();
        let manifest = SoulToml::from_path(&manifest_path)?;

        let project_dir = manifest_path.parent().map(|p| p.to_path_buf()).unwrap_or_default();

        let mut crate_store = CrateStore::new();

        crate_store.insert(
            manifest.package.name.clone(),
            project_dir.join("src"),
            ModuleId::error(),
        );

        for (dep_name, dep) in &manifest.dependencies {
            let dep_path = project_dir.join(&dep.path);
            let dep_src = dep_path.join("src");
            crate_store.insert(
                dep_name.clone(),
                dep_src,
                ModuleId::error(),
            );
        }

        Ok((manifest, crate_store))
    }
}
