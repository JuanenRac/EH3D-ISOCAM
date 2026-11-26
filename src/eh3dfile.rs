// EH3D-ISOCAM - the `.eh3d` project of the suite.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//
//! The `.eh3d` project of the suite: the copper layers are read from the job of the project (the branch `manufacturing/`) and each program of isolation made from one is kept in the
//! branch `isocam/` only, with a copy of the layer and a reference to the state of the job, so that a job that changes makes the programs **out of date**. The reading, the writing and
//! the references are the ones every manufacturing program shares (`eh3d_project::manufacturing` of EH3D-COMMON).

use eh3d_project::appbranch::BranchKit;
use eh3d_project::manufacturing::{self as suite, JobFile};
use std::path::{Path, PathBuf};

/// The branch of this program.
pub const KIT: BranchKit<'static> = suite::kit("isocam", "isocam.json", env!("CARGO_PKG_VERSION"));

/// The copper layers of the job of a project; an error when the job has none.
pub fn copper_layers(path: &Path) -> Result<Vec<JobFile>, String> {
    suite::layers_of(path, "copper")
}

/// The layer of the job called `layer` (its path inside the job or its file name), or the only choice when none is named. A file name that two layers share is refused: the path
/// says which.
pub fn choose<'a>(layers: &'a [JobFile], layer: &str) -> Result<&'a JobFile, String> {
    let shared: Vec<&str> = layers
        .iter()
        .filter(|f| f.file_name() == layer)
        .map(|f| f.rel.as_str())
        .collect();
    if shared.len() > 1 {
        return Err(format!(
            "{layer} is the name of more than one copper layer of the job ({}): give its path",
            shared.join(", ")
        ));
    }
    suite::choose(layers, "copper", layer)
}

/// The name of the program of a layer inside `result/`: `iso-<name of the layer>.<extension>`. When another of the `layers` would get the same name (`board.gtl` and
/// `board.gbl`), the whole path of the layer is kept after `iso+`, so that the program of one layer never takes the place of the program of another.
pub fn result_name(layers: &[JobFile], layer: &JobFile, extension: &str) -> String {
    let short = suite::result_name("iso", layer, extension);
    let shared = layers.iter().any(|other| {
        other.rel != layer.rel
            && suite::result_name("iso", other, extension).to_lowercase() == short.to_lowercase()
    });
    if !shared {
        return short;
    }
    let whole = layer.rel.strip_prefix("layers/").unwrap_or(&layer.rel);
    format!("iso+{}.{extension}", whole.replace('/', "+"))
}

/// Keeps one program made from a layer of the job. The programs of the other layers that are already there stay when they are up to date; when the job changed since they were made they
/// go, because what they were made from is no longer the job. Returns what a person should be told.
pub fn save_program(
    path: &Path,
    layer: &JobFile,
    name: &str,
    bytes: Vec<u8>,
) -> Result<Vec<String>, String> {
    suite::keep_result(&KIT, path, layer, name, bytes)
}

/// A new empty folder of the system to work in; the caller removes it.
pub fn scratch_dir() -> Result<PathBuf, String> {
    suite::scratch_dir("isocam")
}
