// EH3D-ISOCAM - isolation milling of copper from a Gerber layer.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]
//! The copper of a layer is drawn on a fine grid, the exact distance of every point of the board to the copper is worked out, and the paths of the tool are the lines where that
//! distance equals the radius of the tool (and, for more passes, the radius plus one stepover each time). Overlapping tracks and pads need no special care: the grid has no overlaps.

pub mod board;
pub mod browse;
pub mod config;
pub mod crib;
pub mod draw;
pub mod drill;
pub mod dxf;
pub mod filetype;
pub mod gcode;
#[cfg(feature = "gui")]
pub mod gui;
pub mod hpgl;
pub mod lang;
pub mod preview;
pub mod shot;
pub mod viewport;
pub mod window;

// the pure CAM and the raster geometry, shared with the other programs of the suite
pub use eh3d_cam::{
    bitmap, drc, fiducial, fill, forced, isolate, legacy, mask, optimize, order, problems,
    tolerance, transform,
};
pub use eh3d_raster::{contour, edt, units};
