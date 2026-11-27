//! A dry run of a G-code program: the program is walked move by move with the interpreter of the suite and the time it would take is worked out from its feeds, the plunge feed and the
//! speed of the rapid moves (the speeds of the settings, not of a machine), together with the box it moves in, the deepest cut and what is wrong with it: a rapid move into the
//! material, a cut with the spindle off or with no feed, a program that does not end. The report is told in the eight languages of the suite.

use eh3d_parser_gcode::{interpret, parse, Dialect, Motion, Segment, Severity};

/// What is wrong with a program.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// A rapid move that goes down into the material or travels across it.
    RapidIntoMaterial,
    /// A cutting move with the spindle off.
    CutSpindleOff,
    /// A cutting move with no feed set.
    CutNoFeed,
    /// The program has no `M2` or `M30` at the end.
    NotEnded,
    /// A notice of the reader of the program, in English.
    Reader(String),
    /// A limit of the machine that the program breaks, in English.
    Limit(String),
}

/// One finding, at a line of the program (0 when it is about the whole program).
#[derive(Debug, Clone, PartialEq)]
pub struct Warning {
    pub line: usize,
    pub kind: Kind,
}

/// The result of the dry run.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    /// How many moves the program makes.
    pub moves: usize,
    /// The length of the cutting moves, of the rapid moves and of the part of the cutting that goes down into the material, in mm.
    pub cut_mm: f64,
    pub rapid_mm: f64,
    pub plunge_mm: f64,
    /// The time of the cutting moves along the surface, of the moves down into the material and of the rapid moves, in seconds, and the seconds of waiting (`G4`). The rapid time is
    /// only worked out when the speed of the rapid moves is known.
    pub cutting_s: f64,
    pub plunging_s: f64,
    pub rapid_s: f64,
    pub dwell_s: f64,
    /// Whether the speed of the rapid moves was known, so that `rapid_s` counts.
    pub rapid_known: bool,
    /// The stops for the tool to be changed by hand (`M0`, `M1`).
    pub stops: usize,
    /// The box of everything that moved: `(x, y, z)` of its smallest and its largest corner, in mm.
    pub min: (f64, f64, f64),
    pub max: (f64, f64, f64),
    /// The deepest the tool goes below Z 0, as a positive number.
    pub deepest_mm: f64,
    pub warnings: Vec<Warning>,
}

impl Report {
    /// The seconds of the whole job, without the stops for a tool change by hand.
    pub fn total_s(&self) -> f64 {
        self.cutting_s + self.plunging_s + self.rapid_s + self.dwell_s
    }
}

impl Report {
    /// One line with the estimated time and its parts, in English, for the commands that write a program; none when the speed of the rapid moves is not known.
    pub fn time_line(&self) -> Option<String> {
        if !self.rapid_known {
            return None;
        }
        Some(format!(
            "estimated time: {} ({} cutting, {} going down, {} moving); the spindle wait and the tool changes by hand are not counted",
            crate::gcode::duration_text(self.total_s() - self.dwell_s),
            crate::gcode::duration_text(self.cutting_s),
            crate::gcode::duration_text(self.plunging_s),
            crate::gcode::duration_text(self.rapid_s)
        ))
    }
}

/// The dry run of a program with the limits of a machine: what the program breaks of them is one more finding.
pub fn run_checked(text: &str, rapid_mm_min: f64, limits: &crate::machine::Limits) -> Report {
    let mut report = run(text, rapid_mm_min);
    if !limits.is_off() {
        if let Err(why) = crate::machine::check_program(text, limits) {
            report.warnings.push(Warning {
                line: 0,
                kind: Kind::Limit(why),
            });
        }
    }
    report
}

/// The length of a move in mm, arcs by their arc length (with the rise or fall of Z along it).
fn length(s: &Segment) -> f64 {
    let (dx, dy, dz) = (s.to.x - s.from.x, s.to.y - s.from.y, s.to.z - s.from.z);
    let flat = match s.motion {
        Motion::ArcCw { center } | Motion::ArcCcw { center } => {
            let cw = matches!(s.motion, Motion::ArcCw { .. });
            let (ax, ay) = (s.from.x - center.0, s.from.y - center.1);
            let (bx, by) = (s.to.x - center.0, s.to.y - center.1);
            let radius = ax.hypot(ay);
            let mut angle = (ax * by - ay * bx).atan2(ax * bx + ay * by);
            // the angle goes the way of the arc; a start that is the end is a whole circle
            if cw {
                angle = -angle;
            }
            if angle <= 1e-12 {
                angle += std::f64::consts::TAU;
            }
            radius * angle
        }
        _ => dx.hypot(dy),
    };
    flat.hypot(dz)
}

/// Walks the program and tells what it does. `rapid_mm_min` is the speed of the rapid moves (0 when it is not known, which leaves their time out).
pub fn run(text: &str, rapid_mm_min: f64) -> Report {
    let program = parse(text, Dialect::Grbl);
    let trace = interpret(&program);
    let mut r = Report {
        rapid_known: rapid_mm_min.is_finite() && rapid_mm_min > 0.0,
        ..Report::default()
    };
    let mut spinning = false;
    for b in &program.blocks {
        for w in &b.words {
            if w.letter == 'M' && (w.value == 3.0 || w.value == 4.0) {
                spinning = true;
            } else if w.letter == 'M' && [2.0, 5.0, 30.0].contains(&w.value) {
                spinning = false;
            }
        }
        match (b.get('G'), b.get('M')) {
            (Some(4.0), _) => {
                if let Some(p) = b.get('P').filter(|p| p.is_finite() && *p > 0.0) {
                    r.dwell_s += p;
                }
            }
            (_, Some(0.0 | 1.0)) => {
                r.stops += 1;
                // a stop is where the tool is changed by hand
                if spinning {
                    r.warnings.push(Warning {
                        line: b.line,
                        kind: Kind::Reader(
                            "the program stops (M0 or M1) with the spindle on".to_string(),
                        ),
                    });
                }
            }
            _ => {}
        }
    }
    for d in &trace.diagnostics {
        if d.severity >= Severity::Warning {
            r.warnings.push(Warning {
                line: d.line,
                kind: Kind::Reader(d.message.clone()),
            });
        }
    }
    for s in &trace.segments {
        r.moves += 1;
        let len = length(s);
        let rapid = matches!(s.motion, Motion::Rapid);
        let lowest = s.from.z.min(s.to.z);
        r.deepest_mm = r.deepest_mm.max(-lowest);
        if rapid {
            r.rapid_mm += len;
            if r.rapid_known {
                r.rapid_s += len / rapid_mm_min * 60.0;
            }
            if lowest < 0.0 && (s.from.x != s.to.x || s.from.y != s.to.y || s.to.z < s.from.z) {
                r.warnings.push(Warning {
                    line: s.line,
                    kind: Kind::RapidIntoMaterial,
                });
            }
        } else {
            r.cut_mm += len;
            let down = s.from.x == s.to.x && s.from.y == s.to.y && s.to.z < s.from.z;
            if down {
                r.plunge_mm += len;
            }
            if s.feed > 0.0 {
                let seconds = len / s.feed * 60.0;
                if down {
                    r.plunging_s += seconds;
                } else {
                    r.cutting_s += seconds;
                }
            } else {
                r.warnings.push(Warning {
                    line: s.line,
                    kind: Kind::CutNoFeed,
                });
            }
            if !s.spindle_on && lowest < 0.0 {
                r.warnings.push(Warning {
                    line: s.line,
                    kind: Kind::CutSpindleOff,
                });
            }
        }
    }
    if !trace.ended {
        r.warnings.push(Warning {
            line: 0,
            kind: Kind::NotEnded,
        });
    }
    if r.moves > 0 {
        r.min = (trace.min.x, trace.min.y, trace.min.z);
        r.max = (trace.max.x, trace.max.y, trace.max.z);
    }
    r
}

/// A time for a person: `45 s`, `2 min 27 s`, `1 h 2 min`.
fn duration(seconds: f64) -> String {
    crate::gcode::duration_text(seconds)
}

/// The words of the report in one language.
struct Words {
    moves: &'static str,
    cutting: &'static str,
    rapid: &'static str,
    area: &'static str,
    deepest: &'static str,
    time: &'static str,
    time_parts: [&'static str; 4],
    time_unknown: &'static str,
    stops: &'static str,
    warning: &'static str,
    none: &'static str,
    kinds: [&'static str; 4],
}

fn words(lang: &str) -> Words {
    match lang {
        "es" => Words {
            moves: "movimientos",
            cutting: "de corte",
            rapid: "en rápidos",
            area: "recorrido",
            deepest: "corte más profundo",
            time: "tiempo",
            time_parts: ["cortando", "bajando", "moviéndose", "esperando"],
            time_unknown:
                "no se calcula: falta rapid_mm_min, la velocidad de los movimientos rápidos",
            stops: "paradas para cambiar la herramienta a mano, que no cuentan",
            warning: "aviso",
            none: "sin avisos",
            kinds: [
                "un movimiento rápido entra en el material",
                "se corta con el husillo parado",
                "se corta sin avance",
                "el programa no termina con M2 ni M30",
            ],
        },
        "de" => Words {
            moves: "Bewegungen",
            cutting: "Schnitt",
            rapid: "Eilgang",
            area: "Fahrbereich",
            deepest: "tiefster Schnitt",
            time: "Zeit",
            time_parts: ["Schneiden", "Eintauchen", "Bewegen", "Warten"],
            time_unknown: "nicht berechnet: rapid_mm_min, die Geschwindigkeit des Eilgangs, fehlt",
            stops: "Halte für den Werkzeugwechsel von Hand, nicht mitgezählt",
            warning: "Warnung",
            none: "keine Warnungen",
            kinds: [
                "ein Eilgang fährt ins Material",
                "es wird bei stehender Spindel geschnitten",
                "es wird ohne Vorschub geschnitten",
                "das Programm endet nicht mit M2 oder M30",
            ],
        },
        "fr" => Words {
            moves: "mouvements",
            cutting: "de coupe",
            rapid: "en rapides",
            area: "parcours",
            deepest: "coupe la plus profonde",
            time: "durée",
            time_parts: ["en coupe", "en plongée", "en déplacement", "en attente"],
            time_unknown:
                "non calculée : il manque rapid_mm_min, la vitesse des mouvements rapides",
            stops: "arrêts pour changer l'outil à la main, non comptés",
            warning: "avertissement",
            none: "aucun avertissement",
            kinds: [
                "un mouvement rapide entre dans la matière",
                "on coupe avec la broche arrêtée",
                "on coupe sans avance",
                "le programme ne se termine pas par M2 ou M30",
            ],
        },
        "it" => Words {
            moves: "movimenti",
            cutting: "di taglio",
            rapid: "in rapido",
            area: "percorso",
            deepest: "taglio più profondo",
            time: "tempo",
            time_parts: ["tagliando", "scendendo", "spostandosi", "attendendo"],
            time_unknown: "non calcolato: manca rapid_mm_min, la velocità dei movimenti rapidi",
            stops: "arresti per cambiare l'utensile a mano, non contati",
            warning: "avviso",
            none: "nessun avviso",
            kinds: [
                "un movimento rapido entra nel materiale",
                "si taglia con il mandrino fermo",
                "si taglia senza avanzamento",
                "il programma non finisce con M2 o M30",
            ],
        },
        "ja" => Words {
            moves: "動作",
            cutting: "の切削",
            rapid: "の早送り",
            area: "移動範囲",
            deepest: "最も深い切削",
            time: "時間",
            time_parts: ["切削", "下降", "移動", "待機"],
            time_unknown: "計算していません：早送りの速度 rapid_mm_min がありません",
            stops: "手で工具を交換するための停止（数に含めません）",
            warning: "警告",
            none: "警告なし",
            kinds: [
                "早送りが材料の中へ入ります",
                "スピンドルが止まったまま切削します",
                "送り速度なしで切削します",
                "プログラムが M2 または M30 で終わっていません",
            ],
        },
        "zh" => Words {
            moves: "个动作",
            cutting: "切削",
            rapid: "快速移动",
            area: "行程范围",
            deepest: "最深切削",
            time: "时间",
            time_parts: ["切削", "下刀", "移动", "等待"],
            time_unknown: "未计算：缺少快速移动速度 rapid_mm_min",
            stops: "次手动换刀停机，不计入",
            warning: "警告",
            none: "没有警告",
            kinds: [
                "快速移动进入材料",
                "主轴停止时切削",
                "没有进给速度就切削",
                "程序没有以 M2 或 M30 结束",
            ],
        },
        "pt" => Words {
            moves: "movimentos",
            cutting: "de corte",
            rapid: "em rápidos",
            area: "percurso",
            deepest: "corte mais fundo",
            time: "tempo",
            time_parts: ["a cortar", "a descer", "a deslocar-se", "à espera"],
            time_unknown: "não calculado: falta rapid_mm_min, a velocidade dos movimentos rápidos",
            stops: "paragens para mudar a ferramenta à mão, não contadas",
            warning: "aviso",
            none: "sem avisos",
            kinds: [
                "um movimento rápido entra no material",
                "corta-se com o fuso parado",
                "corta-se sem avanço",
                "o programa não termina com M2 nem M30",
            ],
        },
        _ => Words {
            moves: "moves",
            cutting: "of cutting",
            rapid: "of rapid moves",
            area: "travel",
            deepest: "deepest cut",
            time: "time",
            time_parts: ["cutting", "going down", "moving", "waiting"],
            time_unknown: "not worked out: rapid_mm_min, the speed of the rapid moves, is missing",
            stops: "stops for the tool to be changed by hand, not counted",
            warning: "warning",
            none: "no warnings",
            kinds: [
                "a rapid move goes into the material",
                "it cuts with the spindle off",
                "it cuts with no feed",
                "the program does not end with M2 or M30",
            ],
        },
    }
}

impl Warning {
    /// The finding in a language (`en`, `es`, `de`, `fr`, `it`, `ja`, `zh`, `pt`; English for any other), with its line.
    pub fn describe(&self, lang: &str) -> String {
        let w = words(lang);
        let what = match &self.kind {
            Kind::RapidIntoMaterial => w.kinds[0].to_string(),
            Kind::CutSpindleOff => w.kinds[1].to_string(),
            Kind::CutNoFeed => w.kinds[2].to_string(),
            Kind::NotEnded => w.kinds[3].to_string(),
            Kind::Reader(m) | Kind::Limit(m) => m.clone(),
        };
        if self.line > 0 {
            format!("{} {}: {what}", line_word(lang), self.line)
        } else {
            what
        }
    }
}

fn line_word(lang: &str) -> &'static str {
    match lang {
        "es" => "línea",
        "pt" => "linha",
        "it" => "riga",
        "de" => "Zeile",
        "fr" => "ligne",
        "ja" => "行",
        "zh" => "行",
        _ => "line",
    }
}

impl Report {
    /// The report as lines of text in a language; `length` writes a length in mm the way the person reads it.
    pub fn text(&self, lang: &str, length: &dyn Fn(f64) -> String) -> String {
        let w = words(lang);
        let head = program_word(lang);
        let mut t = format!(
            "{}: {} {}, {} {}, {} {}\n",
            head,
            self.moves,
            w.moves,
            length(self.cut_mm),
            w.cutting,
            length(self.rapid_mm),
            w.rapid
        );
        if self.moves > 0 {
            t.push_str(&format!(
                "{}: X {} .. {}, Y {} .. {}, Z {} .. {}; {} {}\n",
                w.area,
                length(self.min.0),
                length(self.max.0),
                length(self.min.1),
                length(self.max.1),
                length(self.min.2),
                length(self.max.2),
                w.deepest,
                length(self.deepest_mm)
            ));
        }
        if self.rapid_known {
            t.push_str(&format!(
                "{}: {} ({} {}, {} {}, {} {}, {} {})\n",
                w.time,
                duration(self.total_s()),
                duration(self.cutting_s),
                w.time_parts[0],
                duration(self.plunging_s),
                w.time_parts[1],
                duration(self.rapid_s),
                w.time_parts[2],
                duration(self.dwell_s),
                w.time_parts[3]
            ));
        } else {
            t.push_str(&format!("{}: {}\n", w.time, w.time_unknown));
        }
        if self.stops > 0 {
            t.push_str(&format!("{} {}\n", self.stops, w.stops));
        }
        if self.warnings.is_empty() {
            t.push_str(w.none);
            t.push('\n');
        }
        for x in &self.warnings {
            t.push_str(&format!("{}: {}\n", w.warning, x.describe(lang)));
        }
        t
    }
}

fn program_word(lang: &str) -> &'static str {
    match lang {
        "es" => "programa",
        "de" => "Programm",
        "fr" => "programme",
        "it" => "programma",
        "ja" => "プログラム",
        "zh" => "程序",
        "pt" => "programa",
        _ => "program",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLEAN: &str = "G21\nG90\nG0 Z2\nM3 S24000\nG4 P2\nG0 X10 Y10\nG1 Z-0.06 F60\nG1 X70 Y10 F300\nG0 Z2\nM5\nM2\n";

    fn mm(v: f64) -> String {
        format!("{v:.3} mm")
    }

    #[test]
    fn a_clean_program_gives_its_lengths_times_and_box() {
        let r = run(CLEAN, 3000.0);
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
        assert_eq!(r.moves, 5);
        // 60 mm along the surface at 300 mm/min is 12 s; 2.06 mm down at 60 mm/min is 2.06 s
        assert!((r.cutting_s - 12.0).abs() < 1e-9, "{}", r.cutting_s);
        assert!((r.plunging_s - 2.06).abs() < 1e-9, "{}", r.plunging_s);
        assert!((r.cut_mm - 62.06).abs() < 1e-9, "{}", r.cut_mm);
        assert!(r.rapid_known && r.rapid_s > 0.0);
        assert!((r.dwell_s - 2.0).abs() < 1e-9);
        assert!((r.deepest_mm - 0.06).abs() < 1e-9);
        assert_eq!((r.min.0, r.max.0, r.max.1), (0.0, 70.0, 10.0));
        assert_eq!(r.stops, 0);
        assert!(r.total_s() > 16.0);
    }

    #[test]
    fn without_the_rapid_speed_the_rapid_time_is_left_out() {
        let r = run(CLEAN, 0.0);
        assert!(!r.rapid_known);
        assert_eq!(r.rapid_s, 0.0);
        let t = r.text("en", &mm);
        assert!(t.contains("rapid_mm_min"), "{t}");
        let t = run(CLEAN, 3000.0).text("en", &mm);
        assert!(
            t.contains("time: ") && t.contains("cutting") && t.contains("no warnings"),
            "{t}"
        );
    }

    #[test]
    fn the_stops_for_a_tool_change_are_counted_and_not_timed() {
        let p = "G21\nG90\nM3 S24000\nG0 X1 Y1\nM5\nM0\nM3 S24000\nG0 X2 Y2\nM5\nM2\n";
        let r = run(p, 3000.0);
        assert_eq!(r.stops, 1);
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
        assert!(r.text("en", &mm).contains("1 stops for the tool"));
    }

    #[test]
    fn what_is_wrong_is_found_with_its_line() {
        let bad =
            "G21\nG90\nG0 X5 Y5\nG0 Z-1\nG0 X9 Y9\nG1 X20 F100\nM3 S1000\nG1 X30\nG1 X40 F0\n";
        let r = run(bad, 3000.0);
        let has = |k: Kind, line: usize| r.warnings.iter().any(|w| w.kind == k && w.line == line);
        assert!(has(Kind::RapidIntoMaterial, 4), "{:?}", r.warnings);
        assert!(has(Kind::RapidIntoMaterial, 5), "{:?}", r.warnings);
        assert!(has(Kind::CutSpindleOff, 6), "{:?}", r.warnings);
        assert!(has(Kind::NotEnded, 0), "{:?}", r.warnings);
        // the spindle is on from line 7: the cut of line 8 is fine
        assert!(!r
            .warnings
            .iter()
            .any(|w| w.kind == Kind::CutSpindleOff && w.line == 8));
    }

    #[test]
    fn a_cut_with_no_feed_is_a_finding() {
        let r = run("G21\nG90\nM3 S1000\nG1 X5\nM2\n", 3000.0);
        assert!(
            r.warnings.iter().any(|w| w.kind == Kind::CutNoFeed),
            "{:?}",
            r.warnings
        );
    }

    #[test]
    fn an_arc_is_measured_by_its_arc_length() {
        // a half circle of radius 10 (clockwise from the left to the right over the top)
        let r = run(
            "G21\nG90\nG0 X0 Y0\nM3 S1000\nG2 X20 Y0 I10 J0 F600\nM2\n",
            3000.0,
        );
        assert!(
            (r.cut_mm - 10.0 * std::f64::consts::PI).abs() < 1e-6,
            "{}",
            r.cut_mm
        );
        assert!((r.cutting_s - 10.0 * std::f64::consts::PI / 600.0 * 60.0).abs() < 1e-6);
        // the same half circle counterclockwise goes under
        let r = run(
            "G21\nG90\nG0 X0 Y0\nM3 S1000\nG3 X20 Y0 I10 J0 F600\nM2\n",
            3000.0,
        );
        assert!(
            (r.cut_mm - 10.0 * std::f64::consts::PI).abs() < 1e-6,
            "{}",
            r.cut_mm
        );
    }

    #[test]
    fn an_empty_program_is_a_program_that_does_not_end() {
        let r = run("", 3000.0);
        assert_eq!(r.moves, 0);
        assert!(r.warnings.iter().any(|w| w.kind == Kind::NotEnded));
        assert!(!r.text("en", &mm).contains("travel"));
    }

    #[test]
    fn the_report_is_told_in_the_eight_languages() {
        let r = run("G21\nG90\nG0 Z-1\nM2\n", 3000.0);
        let mut seen = Vec::new();
        for lang in ["en", "es", "de", "fr", "it", "ja", "zh", "pt"] {
            let t = r.text(lang, &mm);
            assert!(!t.is_empty());
            assert!(t.lines().count() >= 4, "{lang}: {t}");
            seen.push(t);
        }
        // every language says it differently
        for i in 0..seen.len() {
            for j in i + 1..seen.len() {
                assert_ne!(seen[i], seen[j], "{i} and {j} are the same text");
            }
        }
        assert!(r.warnings[0].describe("de").starts_with("Zeile 3: "));
        assert!(r.warnings[0].describe("xx").starts_with("line 3: "));
        let clean = run(CLEAN, 3000.0);
        assert!(clean.text("zh", &mm).contains("没有警告"));
        assert!(clean.text("pt", &mm).contains("sem avisos"));
    }

    #[test]
    fn the_time_line_is_there_only_when_the_rapid_speed_is_known() {
        assert_eq!(run(CLEAN, 0.0).time_line(), None);
        let line = run(CLEAN, 3000.0).time_line().expect("a time");
        assert!(
            line.starts_with("estimated time: ") && line.contains("cutting"),
            "{line}"
        );
    }

    #[test]
    fn the_limits_of_the_machine_are_one_more_finding() {
        let off = crate::machine::Limits {
            table_mm: None,
            margin_mm: 0.0,
            max_depth_mm: 0.0,
            max_rpm: 0.0,
        };
        assert!(run_checked(CLEAN, 3000.0, &off).warnings.is_empty());
        let small = crate::machine::Limits {
            table_mm: Some((20.0, 20.0)),
            ..off
        };
        let r = run_checked(CLEAN, 3000.0, &small);
        assert!(
            r.warnings
                .iter()
                .any(|w| matches!(&w.kind, Kind::Limit(m) if m.contains("outside the table"))),
            "{:?}",
            r.warnings
        );
    }
}
