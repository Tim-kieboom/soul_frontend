use crate::{displayers::displayer_soul_error::ToMessage, frontend, globals};
use anyhow::Result;
use soul_utils::{
    IdAlloc,
    sementic_level::{FaultCollector, ModuleStore, SementicLevel},
};
use std::{fs::OpenOptions, path::PathBuf};

pub(crate) fn faults(faults: &FaultCollector, module_store: &ModuleStore) {
    let mut source_file = String::new();
    let mut current_module = soul_utils::ModuleId::error();

    for fault in &faults.faults {
        let module_id = match fault.get_soul_error().span {
            Some(val) => val.module,
            None => module_store.get_root_id(),
        };

        let path = match module_store.get_path(module_id) {
            Some(val) => val,
            None => &PathBuf::new(),
        };

        if module_id != current_module {
            source_file = frontend::to_source_file(path).unwrap_or_default();
            current_module = module_id;
        }

        logger::error!(
            "{}",
            fault.to_message(path, &source_file, globals::MESSAGE_CONFIG)
        );
    }
}

pub(crate) fn benchmark() -> Result<()> {
    let mut total_times = String::new();

    globals::benchmark()?.write_total(&mut total_times)?;
    logger::info!("{total_times}");
    Ok(())
}

pub(crate) fn init(log_file: &str) -> Result<()> {
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;

    fern::Dispatch::new()
        .format(|out, message, _record| out.finish(*message))
        .level_for("soulc", logger::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(log_file)
        .apply()?;

    Ok(())
}

pub(crate) fn is_fatal(faults: &FaultCollector, fatal_level: SementicLevel) -> bool {
    faults.faults.iter().any(|f| f.is_fatal(fatal_level))
}
