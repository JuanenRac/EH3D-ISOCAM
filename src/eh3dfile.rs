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

/// The layer of the job called `layer` (its path inside the job or its file name), or the only choice when none is named.
pub fn choose<'a>(layers: &'a [JobFile], layer: &str) -> Result<&'a JobFile, String> {
    suite::choose(layers, "copper", layer)
}

/// The name of the program of a layer inside `result/`: `iso-<name of the layer>.<extension>`.
pub fn result_name(layer: &JobFile, extension: &str) -> String {
    suite::result_name("iso", layer, extension)
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
