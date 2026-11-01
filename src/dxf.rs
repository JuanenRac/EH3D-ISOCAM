//! The passes as a DXF drawing (the classic ASCII form, polylines in millimetres), one layer for each pass, so that another program can show or edit the paths.

use crate::isolate::Pass;

fn layer_name(k: usize, p: &Pass) -> String {
    if p.rubout {
        format!("RUBOUT_{}", k + 1)
    } else {
        format!("PASS_{}", k + 1)
    }
}

/// The passes as DXF text. A closed path is a closed polyline, an open run an open one.
pub fn write(passes: &[Pass]) -> Result<String, String> {
    let mut any = false;
    let mut s = String::from("0\nSECTION\n2\nENTITIES\n");
    for (k, p) in passes.iter().enumerate() {
        let layer = layer_name(k, p);
        for cut in p.cuts() {
            if cut
                .pts
                .iter()
                .any(|q| !(q.0.is_finite() && q.1.is_finite()))
            {
                return Err("a path has a coordinate that is not a number".into());
            }
            any = true;
            s.push_str(&format!(
                "0\nPOLYLINE\n8\n{layer}\n66\n1\n70\n{}\n",
                if cut.closed { 1 } else { 0 }
            ));
            for q in &cut.pts {
                s.push_str(&format!(
                    "0\nVERTEX\n8\n{layer}\n10\n{:.4}\n20\n{:.4}\n30\n0.0\n",
                    q.0, q.1
                ));
            }
            s.push_str(&format!("0\nSEQEND\n8\n{layer}\n"));
        }
    }
    if !any {
        return Err("there is no path to write".into());
    }
    s.push_str("0\nENDSEC\n0\nEOF\n");
    Ok(s)
}
