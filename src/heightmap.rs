//! The height map of the table, read from the text file of EH3D-AUTOLEVEL (`levelling 1`, a `grid x0 y0 x1 y1 cols rows` line and one line of heights, in mm, for each row of the grid
//! from the lowest Y up). The two programs share only that file, so neither needs the other to be built. A program is cut so that the depth is kept under the real surface of the
//! table: the Z of every cut is the depth below the height of the map at that point, taken from the height at the place where Z zero was set.

/// The most points in a grid that is read (the file gives the size, so it is checked before anything is reserved for it).
pub const MAX_POINTS: usize = 250_000;

/// The most points along one side of the grid.
pub const MAX_SIDE: usize = 10_000;

/// The heights of the table measured in the points of a grid, all of them.
#[derive(Debug, Clone, PartialEq)]
pub struct HeightMap {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub cols: usize,
    pub rows: usize,
    /// `rows * cols` heights in mm, a row after the other, up is positive.
    pub z: Vec<f64>,
}

impl HeightMap {
    /// Reads the text of a map. A point that was not measured (`-`), a row that is short, a grid that has fewer than 2 points a side or more than the limits, and a number that is not
    /// finite are refused: a program is not cut with half a map.
    pub fn parse(text: &str) -> Result<HeightMap, String> {
        let bad = |why: &str| format!("height map: {why}");
        let mut lines = text.lines();
        if lines.next().map(str::trim) != Some("levelling 1") {
            return Err(bad("the first line is not `levelling 1`"));
        }
        let head: Vec<&str> = lines
            .next()
            .ok_or_else(|| bad("there is no grid line"))?
            .split_whitespace()
            .collect();
        if head.len() != 7 || head[0] != "grid" {
            return Err(bad("the second line is not `grid x0 y0 x1 y1 cols rows`"));
        }
        let float = |s: &str| {
            s.parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(|| bad(&format!("'{s}' is not a number")))
        };
        let whole = |s: &str| {
            s.parse::<usize>()
                .map_err(|_| bad(&format!("'{s}' is not a whole number")))
        };
        let (ax, ay, bx, by) = (
            float(head[1])?,
            float(head[2])?,
            float(head[3])?,
            float(head[4])?,
        );
        let (cols, rows) = (whole(head[5])?, whole(head[6])?);
        if cols < 2 || rows < 2 || cols > MAX_SIDE || rows > MAX_SIDE {
            return Err(bad(&format!(
                "the grid is {cols} x {rows}: each side needs 2 to {MAX_SIDE} points"
            )));
        }
        if !cols.checked_mul(rows).is_some_and(|n| n <= MAX_POINTS) {
            return Err(bad(&format!(
                "the grid is {cols} x {rows}: a map has at most {MAX_POINTS} points"
            )));
        }
        let (x0, x1) = (ax.min(bx), ax.max(bx));
        let (y0, y1) = (ay.min(by), ay.max(by));
        if x1 - x0 < 1e-6 || y1 - y0 < 1e-6 {
            return Err(bad("the area of the grid has no width or no depth"));
        }
        let body: Vec<&str> = lines.filter(|l| !l.trim().is_empty()).collect();
        if body.len() != rows {
            return Err(bad(&format!(
                "the rows are {}, the grid says {rows}",
                body.len()
            )));
        }
        let mut z = Vec::with_capacity(cols * rows);
        for (r, line) in body.iter().enumerate() {
            let cells: Vec<&str> = line.split_whitespace().collect();
            if cells.len() != cols {
                return Err(bad(&format!(
                    "row {} has {} heights, the grid says {cols}",
                    r + 1,
                    cells.len()
                )));
            }
            for cell in cells {
                if cell == "-" {
                    return Err(bad(&format!(
                        "row {} has a point that was not measured: a program is not cut with half a map",
                        r + 1
                    )));
                }
                z.push(float(cell)?);
            }
        }
        Ok(HeightMap {
            x0,
            y0,
            x1,
            y1,
            cols,
            rows,
            z,
        })
    }

    /// The height of the table at `(x, y)` by bilinear interpolation between the four points around it; outside the grid it is the one of the nearest edge.
    pub fn height_at(&self, x: f64, y: f64) -> f64 {
        let fx = ((x - self.x0) / (self.x1 - self.x0) * (self.cols - 1) as f64)
            .clamp(0.0, (self.cols - 1) as f64);
        let fy = ((y - self.y0) / (self.y1 - self.y0) * (self.rows - 1) as f64)
            .clamp(0.0, (self.rows - 1) as f64);
        let c = (fx.floor() as usize).min(self.cols - 2);
        let r = (fy.floor() as usize).min(self.rows - 2);
        let (tx, ty) = (fx - c as f64, fy - r as f64);
        let at = |c: usize, r: usize| self.z[r * self.cols + c];
        let low = at(c, r) * (1.0 - tx) + at(c + 1, r) * tx;
        let high = at(c, r + 1) * (1.0 - tx) + at(c + 1, r + 1) * tx;
        low * (1.0 - ty) + high * ty
    }

    /// The most the table is above or below its height at `(x, y)`, in mm, over the points of the map.
    pub fn deviation_from(&self, x: f64, y: f64) -> f64 {
        let zero = self.height_at(x, y);
        self.z
            .iter()
            .fold(0.0, |m, v| f64::max(m, (v - zero).abs()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table that rises 0.01 mm for every 10 mm in X and is flat in Y: 3 x 2 points over 20 x 10 mm.
    const SLOPE: &str =
        "levelling 1\ngrid 0 0 20 10 3 2\n0.0000 0.0100 0.0200\n0.0000 0.0100 0.0200\n";

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn a_map_of_autolevel_is_read() {
        let m = HeightMap::parse(SLOPE).unwrap();
        assert_eq!((m.cols, m.rows), (3, 2));
        assert_eq!((m.x0, m.y0, m.x1, m.y1), (0.0, 0.0, 20.0, 10.0));
        assert_eq!(m.z, vec![0.0, 0.01, 0.02, 0.0, 0.01, 0.02]);
        // Windows line ends, blank lines and a corner given the other way round
        let crlf = SLOPE.replace('\n', "\r\n");
        assert_eq!(HeightMap::parse(&crlf).unwrap(), m);
        let blanks = SLOPE
            .replacen("20 10 3 2\n", "20 10 3 2\n\n", 1)
            .replace("0.0200\n", "0.0200\n\n");
        assert_eq!(HeightMap::parse(&blanks).unwrap(), m);
        let flipped = "levelling 1\ngrid 20 10 0 0 3 2\n0 0.01 0.02\n0 0.01 0.02\n";
        assert_eq!(HeightMap::parse(flipped).unwrap(), m);
    }

    #[test]
    fn the_height_is_interpolated_and_the_edges_hold_outside() {
        let m = HeightMap::parse(SLOPE).unwrap();
        assert!(near(m.height_at(0.0, 0.0), 0.0));
        assert!(near(m.height_at(10.0, 5.0), 0.01));
        assert!(near(m.height_at(15.0, 3.0), 0.015));
        assert!(near(m.height_at(20.0, 10.0), 0.02));
        // beyond the grid the nearest edge holds
        assert!(near(m.height_at(-50.0, -50.0), 0.0));
        assert!(near(m.height_at(500.0, 500.0), 0.02));
        assert!(near(m.height_at(500.0, 5.0), 0.02));
    }

    #[test]
    fn the_height_in_a_corner_with_a_bump_is_bilinear() {
        let m = HeightMap::parse("levelling 1\ngrid 0 0 10 10 2 2\n0 0\n0 0.4\n").unwrap();
        assert!(near(m.height_at(10.0, 10.0), 0.4));
        assert!(near(m.height_at(5.0, 5.0), 0.1));
        assert!(near(m.height_at(10.0, 5.0), 0.2));
    }

    #[test]
    fn the_deviation_is_taken_from_the_height_at_the_zero() {
        let m = HeightMap::parse(SLOPE).unwrap();
        assert!(near(m.deviation_from(0.0, 0.0), 0.02));
        assert!(near(m.deviation_from(10.0, 5.0), 0.01));
        assert!(near(m.deviation_from(20.0, 0.0), 0.02));
    }

    #[test]
    fn a_damaged_map_is_refused_with_the_reason() {
        for (text, part) in [
            ("", "first line"),
            ("levelling 2\n", "first line"),
            ("levelling 1\n", "no grid line"),
            ("levelling 1\ngrid 0 0 10\n", "second line"),
            (
                "levelling 1\ngrid 0 0 10 10 2 x\n",
                "'x' is not a whole number",
            ),
            (
                "levelling 1\ngrid 0 0 NaN 10 2 2\n0 0\n0 0\n",
                "'NaN' is not a number",
            ),
            ("levelling 1\ngrid 0 0 10 10 1 2\n0\n0\n", "2 to 10000"),
            ("levelling 1\ngrid 0 0 10 10 2 20000\n", "2 to 10000"),
            ("levelling 1\ngrid 0 0 10 10 9000 9000\n", "at most 250000"),
            ("levelling 1\ngrid 5 0 5 10 2 2\n0 0\n0 0\n", "no width"),
            ("levelling 1\ngrid 0 0 10 10 2 2\n0 0\n", "the rows are 1"),
            (
                "levelling 1\ngrid 0 0 10 10 2 2\n0 0\n0\n",
                "row 2 has 1 heights",
            ),
            (
                "levelling 1\ngrid 0 0 10 10 2 2\n0 0\n0 -\n",
                "not measured",
            ),
            (
                "levelling 1\ngrid 0 0 10 10 2 2\n0 0\n0 inf\n",
                "'inf' is not a number",
            ),
        ] {
            let e = HeightMap::parse(text).unwrap_err();
            assert!(
                e.starts_with("height map: ") && e.contains(part),
                "{text:?} gave {e:?}"
            );
        }
    }
}
