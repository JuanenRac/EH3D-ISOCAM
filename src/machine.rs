//! The machine a program is for: the size of its table, the margin kept from its edges, the deepest cut and the fastest spindle it may be sent. A program is walked move by move with the
//! interpreter of the suite before it is written, and the first move that breaks a limit stops it with the line, so that nothing is saved that would drive the tool off the table or
//! into it. The sizes of the profiles are the ones written in the documents of EH3D-DRILLPRO; the origin of the table is taken at a corner, as the programs of the suite place the
//! work. Nothing here has run on a machine.

use eh3d_parser_gcode::{interpret, parse, Dialect};

/// A machine whose table is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Profile {
    /// No profile: the table is whatever `machine_x_mm` and `machine_y_mm` say, or not checked at all.
    #[default]
    Off,
    /// The Bungard CCD MTC: 495 x 325 mm.
    CcdMtc,
    /// The Bungard CCD/2 MTC: 325 x 270 mm.
    Ccd2Mtc,
}

impl Profile {
    /// Every profile, in the order they are listed in messages.
    pub const ALL: [Profile; 3] = [Profile::Off, Profile::CcdMtc, Profile::Ccd2Mtc];

    /// The profile of a setting value (`none`, `ccd-mtc`, `ccd2-mtc`; case, `_` and `-` do not matter).
    pub fn parse(v: &str) -> Result<Profile, String> {
        let key = v.trim().to_ascii_lowercase().replace('_', "-");
        match key.as_str() {
            "" | "none" | "off" | "no" => Ok(Profile::Off),
            "ccd-mtc" | "ccd" => Ok(Profile::CcdMtc),
            "ccd2-mtc" | "ccd/2-mtc" | "ccd2" => Ok(Profile::Ccd2Mtc),
            _ => Err(format!(
                "machine_profile '{}' must be none, ccd-mtc or ccd2-mtc",
                v.trim()
            )),
        }
    }

    /// The word the setting is written with.
    pub fn name(self) -> &'static str {
        match self {
            Profile::Off => "none",
            Profile::CcdMtc => "ccd-mtc",
            Profile::Ccd2Mtc => "ccd2-mtc",
        }
    }

    /// The table of the machine, X by Y in mm; `None` for no profile.
    pub fn table_mm(self) -> Option<(f64, f64)> {
        match self {
            Profile::Off => None,
            Profile::CcdMtc => Some((495.0, 325.0)),
            Profile::Ccd2Mtc => Some((325.0, 270.0)),
        }
    }
}

/// What a program may not go beyond. A limit that is 0 (or has no table) is off.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Limits {
    /// The table, X by Y in mm, from the origin at a corner; `None` does not check the travel.
    pub table_mm: Option<(f64, f64)>,
    /// The distance kept from every edge of the table.
    pub margin_mm: f64,
    /// The deepest the tool may go below the surface (a positive number); 0 does not check it.
    pub max_depth_mm: f64,
    /// The fastest spindle speed; 0 does not check it.
    pub max_rpm: f64,
}

impl Limits {
    /// True when no limit is on, so that there is nothing to check.
    pub fn is_off(&self) -> bool {
        self.table_mm.is_none() && self.max_depth_mm <= 0.0 && self.max_rpm <= 0.0
    }

    /// The limits in a few words, for the screen.
    pub fn describe(&self) -> String {
        let mut parts = Vec::new();
        if let Some((x, y)) = self.table_mm {
            parts.push(format!(
                "table {x} x {y} mm, {} mm kept from its edges",
                self.margin_mm
            ));
        }
        if self.max_depth_mm > 0.0 {
            parts.push(format!("cut at most {} mm deep", self.max_depth_mm));
        }
        if self.max_rpm > 0.0 {
            parts.push(format!("spindle at most {} rpm", self.max_rpm));
        }
        if parts.is_empty() {
            "no limits".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// What a program that kept every limit did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Checked {
    /// How many moves the program makes.
    pub moves: usize,
}

/// Walks the program and stops at the first move that goes beyond a limit, saying the line, where the tool was going and how many moves had been made before it.
pub fn check_program(text: &str, limits: &Limits) -> Result<Checked, String> {
    let trace = interpret(&parse(text, Dialect::Grbl));
    let eps = 1e-6;
    for (made, s) in trace.segments.iter().enumerate() {
        // a move up or down in Z does not travel on the table, wherever the tool stands
        let travels = s.to.x != s.from.x || s.to.y != s.from.y;
        if let (Some((w, d)), true) = (limits.table_mm, travels) {
            let m = limits.margin_mm;
            let (x, y) = (s.to.x, s.to.y);
            if x < m - eps || y < m - eps || x > w - m + eps || y > d - m + eps {
                return Err(format!(
                    "line {}: the tool goes to X{x:.3} Y{y:.3}, outside the table ({w} x {d} mm, {m} mm kept from its edges) after {made} moves",
                    s.line
                ));
            }
        }
        if limits.max_depth_mm > 0.0 && s.to.z < -limits.max_depth_mm - eps {
            return Err(format!(
                "line {}: the tool goes down to Z{:.3}, deeper than the {} mm the machine may cut after {made} moves",
                s.line, s.to.z, limits.max_depth_mm
            ));
        }
        if limits.max_rpm > 0.0 && s.spindle_on && s.spindle_speed > limits.max_rpm + eps {
            return Err(format!(
                "line {}: the spindle runs at {} rpm, faster than the {} rpm the machine may be sent after {made} moves",
                s.line, s.spindle_speed, limits.max_rpm
            ));
        }
    }
    Ok(Checked {
        moves: trace.segments.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(w: f64, d: f64) -> Limits {
        Limits {
            table_mm: Some((w, d)),
            ..Limits::default()
        }
    }

    const FITS: &str = "G21\nG90\nG0 Z2\nM3 S24000\nG0 X10 Y10\nG1 Z-0.06 F60\nG1 X50 Y10 F300\nG1 X50 Y40\nG0 Z2\nM5\nM2\n";

    #[test]
    fn the_profiles_are_read_by_their_names_and_written_back() {
        for p in Profile::ALL {
            assert_eq!(Profile::parse(p.name()), Ok(p));
        }
        assert_eq!(Profile::parse(" CCD_MTC "), Ok(Profile::CcdMtc));
        assert_eq!(Profile::parse("ccd/2-mtc"), Ok(Profile::Ccd2Mtc));
        assert_eq!(Profile::parse(""), Ok(Profile::Off));
        assert!(Profile::parse("ccd3")
            .unwrap_err()
            .contains("machine_profile 'ccd3'"));
        assert_eq!(Profile::CcdMtc.table_mm(), Some((495.0, 325.0)));
        assert_eq!(Profile::Ccd2Mtc.table_mm(), Some((325.0, 270.0)));
        assert_eq!(Profile::Off.table_mm(), None);
        assert_eq!(Profile::default(), Profile::Off);
    }

    #[test]
    fn a_program_inside_the_table_passes_and_counts_its_moves() {
        let c = check_program(FITS, &table(100.0, 60.0)).unwrap();
        assert!(c.moves >= 6, "{c:?}");
    }

    #[test]
    fn the_first_move_off_the_table_stops_the_program_with_its_line() {
        let e = check_program(FITS, &table(40.0, 60.0)).unwrap_err();
        assert!(e.starts_with("line 7: "), "{e}");
        assert!(e.contains("X50.000 Y10.000"), "{e}");
        let e = check_program(FITS, &table(100.0, 30.0)).unwrap_err();
        assert!(e.starts_with("line 8: "), "{e}");
    }

    #[test]
    fn the_margin_is_kept_from_every_edge() {
        let mut l = table(100.0, 60.0);
        l.margin_mm = 15.0;
        // X10 is nearer to the edge than the margin
        let e = check_program(FITS, &l).unwrap_err();
        assert!(e.starts_with("line 5: "), "{e}");
        l.margin_mm = 10.0;
        assert!(check_program(FITS, &l).is_ok());
        l.margin_mm = 11.0;
        assert!(check_program(FITS, &l).is_err());
    }

    #[test]
    fn a_program_in_inches_is_checked_in_millimetres() {
        let inch = "G20\nG90\nG0 X1 Y1\nG1 Z-0.002 F5\nG1 X3 Y1 F10\nM2\n";
        // 3 in is 76.2 mm
        assert!(check_program(inch, &table(80.0, 40.0)).is_ok());
        assert!(check_program(inch, &table(76.0, 40.0)).is_err());
    }

    #[test]
    fn relative_moves_are_followed() {
        let rel = "G21\nG91\nG1 X30 F300\nG1 X30\nG1 X30\n";
        assert!(check_program(rel, &table(100.0, 60.0)).is_ok());
        let e = check_program(rel, &table(80.0, 60.0)).unwrap_err();
        assert!(e.starts_with("line 5: "), "{e}");
    }

    #[test]
    fn the_depth_and_the_speed_of_the_spindle_are_limited() {
        let deep = Limits {
            max_depth_mm: 0.05,
            ..Limits::default()
        };
        let e = check_program(FITS, &deep).unwrap_err();
        assert!(e.starts_with("line 6: ") && e.contains("Z-0.060"), "{e}");
        let ok = Limits {
            max_depth_mm: 0.06,
            ..Limits::default()
        };
        assert!(check_program(FITS, &ok).is_ok());
        let fast = Limits {
            max_rpm: 20000.0,
            ..Limits::default()
        };
        let e = check_program(FITS, &fast).unwrap_err();
        assert!(e.contains("24000 rpm"), "{e}");
        let fast = Limits {
            max_rpm: 24000.0,
            ..Limits::default()
        };
        assert!(check_program(FITS, &fast).is_ok());
    }

    #[test]
    fn with_no_limit_on_anything_goes_and_the_limits_describe_themselves() {
        let off = Limits::default();
        assert!(off.is_off());
        assert!(check_program("G21\nG0 X9999 Y9999\nG1 Z-9 F1\n", &off).is_ok());
        assert_eq!(off.describe(), "no limits");
        let l = Limits {
            table_mm: Some((495.0, 325.0)),
            margin_mm: 5.0,
            max_depth_mm: 0.5,
            max_rpm: 30000.0,
        };
        assert!(!l.is_off());
        let d = l.describe();
        assert!(
            d.contains("495 x 325") && d.contains("0.5 mm deep") && d.contains("30000 rpm"),
            "{d}"
        );
    }
}
