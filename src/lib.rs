// EH3D-ISOCAM - isolation milling of copper from a Gerber layer.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]
//! The copper of a layer is drawn on a fine grid, the exact distance of every point of the board to the copper is worked out, and the paths of the tool are the lines where that
//! distance equals the radius of the tool (and, for more passes, the radius plus one stepover each time). Overlapping tracks and pads need no special care: the grid has no overlaps.

pub mod bitmap;
pub mod contour;
pub mod edt;
pub mod gcode;
pub mod hpgl;
pub mod isolate;
