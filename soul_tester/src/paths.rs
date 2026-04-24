use anyhow::Result;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use soul_utils::{CrateStore, IdAlloc, ModuleId, SoulToml};

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Paths {
    pub log_file: String,
    pub project: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EntryFile {
    pub path: PathBuf,
    pub is_lib: bool,
}

impl Paths {
    pub fn to_source_path(project: &Path) -> Result<PathBuf> {
        let mut manifest = PathBuf::from(project);
        manifest.push("src");

        check_pathbuf(&manifest)?;

        Ok(manifest)
    }

    pub fn to_entry_file_path(project: &Path) -> Result<EntryFile> {
        let mut path = Self::to_source_path(project)?;
        let mut is_lib = false;
        path.push("main.soul");
        if !path.exists() {
            path.pop();
            path.push("lib.soul");
            is_lib = true;
        }

        if !path.exists() {
            path.pop();
            Err(anyhow::Error::msg(format!(
                "in src '{:?}' 'main.soul' or 'lib.soul' is not found",
                path
            )))
        } else {
            Ok(EntryFile { path, is_lib })
        }
    }

    pub fn project_path(&self) -> &Path {
        Path::new(&self.project)
    }

    pub fn write_to_output(
        output_file: &str,
        project: &Path,
        relative_file: &Path,
    ) -> Result<()> {
        let path = project.join("output").join(relative_file);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = File::create(path)?;
        file.write_all(output_file.as_bytes())?;
        Ok(())
    }

    pub fn load_crates(&self) -> Result<(SoulToml, CrateStore)> {
        let manifest_path = self.project_path().join("Soul.toml");
        let manifest = SoulToml::from_path(&manifest_path)?;

        let project_dir = self.project_path();
        let mut crate_store = CrateStore::new();

        crate_store.insert(
            manifest.package.name.clone(),
            project_dir.to_path_buf(),
            ModuleId::error(),
        );

        for (dep_name, dep) in &manifest.dependencies {
            let dep_src = project_dir.join(&dep.path);
            crate_store.insert(dep_name.clone(), dep_src, ModuleId::error());
        }

        Ok((manifest, crate_store))
    }
}

fn check_pathbuf(path: &PathBuf) -> std::io::Result<()> {
    std::fs::metadata(&path)?;
    Ok(())
}
