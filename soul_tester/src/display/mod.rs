use anyhow::Result;
use soul_utils::collections::vec_map::{VecMap, VecMapIndex};
use std::{fmt::Debug, fs::{File, OpenOptions}, io::Write, path::Path};

use crate::display::writer::Writer;

pub(crate) mod ast;
pub(crate) mod fault;
pub(crate) mod tokenizer;
pub(crate) mod benchmark;
pub mod writer;

fn write_to_file(path: &Path, str: &str) -> Result<()> {
    let mut file = write_create_file(path)?;
    file.push_str(str)?;
    file.flush()?;
    Ok(())
}

fn write_create_file(path: &Path) -> Result<File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .map_err(|err| anyhow::anyhow!("Failed to create output file({path:?}): {}", err))
}

#[derive(serde::Serialize, serde::Deserialize)]
struct VecMapEntry<V> {
    id: String,
    value: V,
}

fn vec_map_to_pretty_vec<K: VecMapIndex + Debug, V>(map: &VecMap<K, V>) -> Vec<VecMapEntry<&V>>{
    map.entries().map(|(id, value)| VecMapEntry{id: format!("{id:?}"), value}).collect()
}