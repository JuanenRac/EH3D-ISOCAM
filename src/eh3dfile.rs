// EH3D-ISOCAM - the `.eh3d` project of the suite.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//
//! The `.eh3d` project of the suite: the copper layers are read from the job of the project (the branch `manufacturing/`) and each program of isolation made from one is kept in the
//! branch `isocam/` only, with a copy of the layer and a reference to the state of the job, so that a job that changes makes the programs **out of date**. The reading, the writing and
//! the references are the ones of the Gerber viewer (`eh3d_gerberviewer::eh3dfile`).

use eh3d_gerberviewer::eh3dfile::{self as suite, JobFile, FORMAT};
use eh3d_project::appbranch::BranchKit;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The branch of this program.
pub const KIT: BranchKit<'static> =
    BranchKit::new("isocam", FORMAT, "isocam.json", env!("CARGO_PKG_VERSION"));

/// The copper layers of the job of a project; an error when the job has none.
pub fn copper_layers(path: &Path) -> Result<Vec<JobFile>, String> {
    let layers = suite::read_job(path, &["copper"])?;
    if layers.is_empty() {
        return Err(format!("{}: the job has no copper layer", path.display()));
    }
    Ok(layers)
}

/// The layer of the job called `layer` (its path inside the job or its file name), or the only choice when none is named.
pub fn choose<'a>(layers: &'a [JobFile], layer: &str) -> Result<&'a JobFile, String> {
    layers
        .iter()
        .find(|f| f.rel == layer || f.file_name() == layer)
        .ok_or_else(|| {
            let names: Vec<&str> = layers.iter().map(|f| f.rel.as_str()).collect();
            format!(
                "the job has no copper layer {layer}; it has {}",
                names.join(", ")
            )
        })
}

/// The name of the program of a layer inside `result/`: `iso-<name of the layer>.<extension>`.
pub fn result_name(layer: &JobFile, extension: &str) -> String {
    let file = layer.file_name();
    let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    format!("iso-{stem}.{extension}")
}

/// Keeps one program made from a layer of the job. The programs of the other layers that are already there stay when they are up to date; when the job changed since they were made they
/// go, because what they were made from is no longer the job. Returns what a person should be told.
pub fn save_program(
    path: &Path,
    layer: &JobFile,
    name: &str,
    bytes: Vec<u8>,
) -> Result<Vec<String>, String> {
    let state = suite::state(&KIT, path)?;
    let mut inputs = vec![layer.clone()];
    let mut results = vec![(name.to_string(), bytes)];
    if !state.out_of_date {
        let job = suite::read_job(path, &[])?;
        inputs.extend(
            job.into_iter()
                .filter(|f| f.rel != layer.rel && state.inputs.contains(&f.rel)),
        );
        for other in state.results.iter().filter(|r| r.as_str() != name) {
            results.push((other.clone(), suite::read_result(&KIT, path, other)?));
        }
    }
    suite::save_result(&KIT, path, &inputs, &results)
}

/// A new empty folder of the system to work in; the caller removes it.
pub fn scratch_dir() -> Result<PathBuf, String> {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "eh3d-isocam-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(dir)
}
