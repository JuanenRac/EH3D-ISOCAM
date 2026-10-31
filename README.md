<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-ISOCAM banner" width="100%">
</p>

# 📖 EH3D-ISOCAM

<p align="center">🇺🇸 <b>English</b> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Isolation milling of copper from a Gerber layer: exact offsets, several passes and gap checks

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Honesty check - what actually runs today:** a Rust library and a command line program (`eh3d-isocam`) with 60 tests that run on a computer: traces the paths of one or several milling tools around the copper of a Gerber layer, clears the leftover copper, chooses the direction, the starting points and the order of the cuts, reads and writes drill files and board outlines, lays a board out for both sides and for a panel, checks the design rules and writes G-code or HP-GL. It has no window yet, it has not been tried on a machine and it has not been checked against a collection of real files.

---

> **Safety and security status:** this program does not control any machine and is not a safety system. What it produces must be checked by a person before anything is run on a machine; nothing here has run on real hardware.


## 📋 At a glance

| | |
|---|---|
| Version | `0.0.1` |
| Maturity | Scaffolding |
| Tests | 60 |
| Stack | Rust |
| On real hardware | Not tried: nothing here has run on a machine |

**Jump to:** [Overview](#-overview) · [Architecture](#-architecture) · [Quick start](#-quick-start) · [Testing](#-testing) · [Structure](#-repository-structure) · [Related](#-related-projects)

## 🚦 Project status

Each line is one part of this repository and the light says how far it is today. It is written from what the repository can show (tests, builds, runs), not from what is planned.

🟢 Done and verified · 🟡 Works, with known limits · 🟠 Partly done or not verified yet · 🔴 Not done or not working

| Area | State | What it means here |
|---|---|---|
| Core | 🟡 | traces the paths of one or several milling tools around the copper of a Gerber layer, clears the leftover copper, chooses the direction, the starting points and the order of the cuts, reports the gaps that are too narrow for the tool and writes G-code or HP-GL; tested on a computer with normal and damaged input, not with a collection of real files. |
| Files and layout | 🟡 | reads and writes drill files and board outlines, lays a board out for both sides and for a panel, adds alignment holes and checks the design rules with the place of every finding; tested on a computer, not with files written by other programs and not on a machine. |
| Command line | 🟡 | `eh3d-isocam` runs the same functions as the library and prints the result; every error comes back as a message and a non-zero exit code. |
| Tests | 🟡 | 60 tests pass (`cargo test`), with normal cases, edge cases and damaged input. |
| Security | 🟡 | Plain safe Rust (`forbid(unsafe_code)`): nothing panics on the files it is given and a bad value is an error with its reason; no independent review. |
| Window | 🔴 | Planned: the egui window common to the suite; today there is only the command line. |
| Real hardware | 🔴 | Not tried: nothing here has been run on a machine. |
| Documentation | 🟡 | README in eight languages, the changelog and the two pictures drawn from `docs/`. |

## 🎯 Overview

* **The copper on a grid** (`src/bitmap.rs`): pads (round, rectangular, obround), tracks and regions are drawn on cells of the size you choose; overlapping tracks and pads are one piece because a grid has no overlaps, so no special case is needed; a grid that would not fit in memory is refused.
* **The exact distance** (`src/edt.rs`, `src/contour.rs`): the distance from every cell to the nearest copper is worked out exactly (and tested against a brute force), and the path of a pass is the line where it equals the radius of the tool, traced as closed polylines and made lighter with a tolerance of half a cell; further passes go one stepover farther each.
* **What the tool cannot do** (`src/isolate.rs`): where two separate pieces of copper are closer than the width of the tool, no path can go between them: `check` lists those places, and `isolate` does not write the G-code until they are looked at (or `--force`). The G-code (`src/gcode.rs`) lifts the tool between paths, goes down to the depth chosen and cuts the nearest path next.
* **HP-GL for the Bungard machines** (`src/hpgl.rs`): with an output file that ends in `.plt` the passes are written as an HP-GL plot file at 40 units to the millimetre, one pen for each pass, which is what the routing program of the Bungard machines reads.
* **Several tools and leftover copper** (`src/isolate.rs`): a plan has up to four tools and the widest cuts first; where a wider tool has already cut along a path of a narrower one, that stretch is left out (the rest stays as open runs), and rings of a wider tool, each overlapping the one before, can clear the copper that is left beyond the passes; the G-code stops for a tool change when the width of the tool changes.
* **Direction, start and order** (`src/order.rs`): the paths are traced with the copper on the right of the tool (climb milling with a spindle that turns clockwise); the settings can turn that round (conventional), force clockwise or counterclockwise, start each closed path at its nearest point, its lowest point or where it was traced, set the point the tool starts from and put the paths in the order they were traced, nearest first, or shortened further by turning stretches of the order round; `info` shows how many paths go each way and the travel through the air.
* **Holes and board outline** (`src/drill.rs`): drill files are read (Bungard files in mils with the reader of the Bungard formats, metric files with the reader of the suite) and written in whole mils; an outline is read from a Gerber outline layer, a DXF drawing or an HP-GL plot file, strokes drawn one by one are joined, and the path of the tool round the board and inside its cutouts is written as HP-GL or as a routing Excellon file.
* **Both sides, panels and alignment holes** (`src/transform.rs`): the layer, the holes and the outline are turned over, turned by quarter turns and repeated on a panel together; alignment holes are put outside the board where the turned-over board finds them again (a third one on the line tells it from the same board turned half a turn).
* **Design rules** (`src/drc.rs`): copper narrower than the rule, pieces of copper closer than the rule or than the tool is wide, and holes with less copper round them than the rule, each with its place in millimetres.
* **One settings file** (`src/config.rs`): every value that can be tuned is a setting with a default, read from a `key = value` file (`examples/isocam.cfg`) or given by the same name as an option of the command line.

## 🧩 Role in EH3D

EH3D-ISOCAM is one program of the EH3D Fabrication Suite. Today it works as a library and a command line; its window, in the look the other programs of the suite share, is planned. It reads and writes plain files, so it can be used on its own or between the other programs.

## 🏛️ Architecture

<p align="center">
  <img src="images/architecture.svg" alt="EH3D-ISOCAM architecture: who uses it, what it holds and what it depends on" width="100%">
</p>


How the parts of this repository fit together: who uses it, what it holds and what it depends on. The picture is drawn from `docs/architecture.json` and checked against the code.

## ✨ Features

* **A library and a command line:** every command of `eh3d-isocam` is a function of the library, so another program of the suite can call it without starting a process.
* **Tests with damaged input:** the tests also feed empty files, garbage, huge numbers and cut files, and none of them may panic.
* **No guessing:** a value that does not make sense is an error that says which value and why, never a result that only looks right.
* **No unsafe code:** `#![forbid(unsafe_code)]` in the library and the program.
* **Every tuned value is a setting:** nothing that a person may want to change is hidden in the code; a bad value is an error with its line.

## 🔒 Security & Safety

| Threat | Defense |
|---|---|
| A damaged, truncated or empty file | no panic: the tests cut valid input at every position and feed garbage |
| A number that makes no sense (negative, not finite, absurdly large) | refused with an error that names the value; nothing is rounded into something plausible |
| Unsafe memory use | plain safe Rust with `forbid(unsafe_code)` |

**What is not defended:** a result that was produced cleanly is not a verified result. Nothing here knows the limits of a machine, and the program has had no independent review. Report a vulnerability privately (see [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

The library is `eh3d_isocam` (`use eh3d_isocam::...`); the command line is `eh3d-isocam`. Each command prints its result on the standard output and reports an error on the standard error with a non-zero exit code. There is no network access. The text of the help (`eh3d-isocam help`) is in English:

```text
eh3d-isocam isolate LAYER.gtl OUT.nc|OUT.plt     paths round the copper (G-code, or HP-GL for a .plt)
eh3d-isocam check LAYER.gtl                      places where the tool does not fit between pieces of copper
eh3d-isocam info LAYER.gtl                       passes, paths each way round, travel through the air
eh3d-isocam drc LAYER.gtl [--drills F.ncd]       design rules, with the place of every finding
eh3d-isocam drills IN.ncd OUT.ncd [--align]      read, turn over, panel and add alignment holes
eh3d-isocam route OUTLINE OUT.plt|OUT.ncd        path of the tool round the board
eh3d-isocam config                               every setting with its default, as a file
options: --config FILE  --outline FILE  --drills FILE  --force  and every setting by its name (--fine-tool-mm 0.15)
```

```text
EH3D-ISOCAM/
├── src/
│   ├── bitmap.rs                                the copper on a grid
│   ├── config.rs                                every tunable value, from one settings file
│   ├── contour.rs                               the paths of a pass, traced and made lighter
│   ├── drc.rs                                   design rules: widths, gaps and rings, with places
│   ├── drill.rs                                 holes and board outline, read and written
│   ├── edt.rs                                   the exact distance to the copper
│   ├── gcode.rs                                 the paths as G-code
│   ├── hpgl.rs                                  the paths as an HP-GL plot file
│   ├── isolate.rs                               tools, passes, leftover copper and narrow gaps
│   ├── lib.rs                                   the library: what every command uses
│   ├── main.rs                                  the command line
│   ├── order.rs                                 direction, starting points and order of the cuts
│   └── transform.rs                             both sides, panels and alignment holes
├── tests/
│   ├── files.rs                                 tests of the drill and outline files
│   ├── isocam.rs                                tests of the library and the command line
│   ├── layout.rs                                tests of both sides, panels, rules and the command line
│   └── multi.rs                                 tests of tools, rubout, direction, order and settings
├── examples/
│   ├── isocam.cfg                               every setting with its default, to copy and change
│   └── sample.gtl                               a sample to try the program with
├── docs/
│   ├── architecture.json                        data of the Architecture picture
│   └── structure.json                           data of the Repository Structure picture
├── tools/                                       CI check, README check and the picture tools
├── images/                                      banner and the Architecture and Repository Structure pictures
├── .github/                                     CI and release workflows, issue and pull-request templates
├── eh3d.project.json                            manifest: version, role, maturity and build command
├── Cargo.toml Cargo.lock                        the workspace and its locked dependencies
├── CHANGELOG.md                                 what changed in each version
├── SECURITY.md SUPPORT.md                       how to report a vulnerability and where to ask
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   how to contribute, the code of conduct and the licence
```

## ⚙️ Configuration

Every value that can be tuned is a setting with a default: the diameters, passes and stepover of the fine and the coarse tool, the removal of the leftover copper, the depth, feed and speed, the direction, the starting point, the start of each path and the ordering, the tool that cuts out the board, the alignment holes, the mirror, turn and panel, and the design rules. `eh3d-isocam config` prints them all as a file (`examples/isocam.cfg` is that file); `--config FILE` reads one, and any setting can be given on the command line as an option with the same name (`--fine-tool-mm 0.15`), which wins over the file. A value that makes no sense is an error that says the line. The fixed limits live in the code as named constants and are covered by tests.

## 🚀 Quick start

You need a stable Rust toolchain (1.88 or newer).

```bash
cargo build --release
cargo test
cargo run --release -- isolate examples/sample.gtl examples/sample.nc --passes 2
```

## 🧪 Testing

The suite has 60 tests (`cargo test`, run on Windows and Linux in the CI). They check the normal cases, the edge cases and damaged input: empty files, garbage and cut files never panic. The CI also runs `cargo fmt --check`, `cargo clippy` with warnings as errors and `python -I tools/ci_validate.py`.

## 🏗️ Build & Release

The version lives in `Cargo.toml` and in `eh3d.project.json`; each version has its entry in [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` build the program, `test.bat` / `test.sh` run the tests and `run.bat` / `run.sh` run it with a sample. Releases are published on GitHub.

## 📂 Repository Structure

<p align="center">
  <img src="images/repository-structure.svg" alt="Repository structure of EH3D-ISOCAM" width="100%">
</p>

<details>
<summary>Text version of the tree</summary>

```text
EH3D-ISOCAM/
├── src/
│   ├── bitmap.rs                                the copper on a grid
│   ├── config.rs                                every tunable value, from one settings file
│   ├── contour.rs                               the paths of a pass, traced and made lighter
│   ├── drc.rs                                   design rules: widths, gaps and rings, with places
│   ├── drill.rs                                 holes and board outline, read and written
│   ├── edt.rs                                   the exact distance to the copper
│   ├── gcode.rs                                 the paths as G-code
│   ├── hpgl.rs                                  the paths as an HP-GL plot file
│   ├── isolate.rs                               tools, passes, leftover copper and narrow gaps
│   ├── lib.rs                                   the library: what every command uses
│   ├── main.rs                                  the command line
│   ├── order.rs                                 direction, starting points and order of the cuts
│   └── transform.rs                             both sides, panels and alignment holes
├── tests/
│   ├── files.rs                                 tests of the drill and outline files
│   ├── isocam.rs                                tests of the library and the command line
│   ├── layout.rs                                tests of both sides, panels, rules and the command line
│   └── multi.rs                                 tests of tools, rubout, direction, order and settings
├── examples/
│   ├── isocam.cfg                               every setting with its default, to copy and change
│   └── sample.gtl                               a sample to try the program with
├── docs/
│   ├── architecture.json                        data of the Architecture picture
│   └── structure.json                           data of the Repository Structure picture
├── tools/                                       CI check, README check and the picture tools
├── images/                                      banner and the Architecture and Repository Structure pictures
├── .github/                                     CI and release workflows, issue and pull-request templates
├── eh3d.project.json                            manifest: version, role, maturity and build command
├── Cargo.toml Cargo.lock                        the workspace and its locked dependencies
├── CHANGELOG.md                                 what changed in each version
├── SECURITY.md SUPPORT.md                       how to report a vulnerability and where to ask
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   how to contribute, the code of conduct and the licence
```

</details>

## 🗺️ Roadmap & Known limits

**Known limits today**

* It has no window yet: only the library and the command line exist.
* It has not been tried on a machine nor with a collection of real files, and it has had no independent review.
* The paths are exact to half a cell, not to the curve itself; there is no Voronoi strategy, no forced isolation where the tool does not fit (those places are only reported), no scaling or compensation of the board, no solder mask or silkscreen layers, and no board has been milled with these paths.
* The removal of the leftover copper is rings of one wider tool at a distance from the copper: places narrower than that tool keep their copper, and it does not follow the edge of the board.
* The stretches left out for a narrower tool are decided on a grid with two cells of margin; it prefers cutting a stretch twice to leaving it uncut.
* Drill files are written in whole mils (the convention of the Bungard programs): a diameter `T1C0.8` with no unit is read as millimetres from 0.2 up and as inches below it; files with slots or routed paths have those left out, with a note. The files written have only been read back by the readers of this suite, not by other programs or machines.
* The design rules do not round off the corners of a square pad and do not check the copper against the board edge or the solder mask; the check of the rings takes the pad as drawn in the Gerber layer.

**Next**

* Add the window in the look shared by the programs of the suite, in the eight languages, with a preview of the passes and units in mils, millimetres and inches.
* Isolation forced where the tool does not fit (a narrower pass for that stretch, with a warning) and the Voronoi strategy; tidy the arcs; scaling and compensation of the board from the fiducials of the camera.
* Solder mask and silkscreen layers.
* Compare the paths with boards that were really milled and try it with real files and with the other programs of the suite.

## 🌍 Languages

README and texts exist in English, Spanish, French, Italian, German, Chinese, Japanese and European Portuguese.

## 🔗 Related Projects

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): the readers of Gerber, Excellon and G-code
* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): the file formats of the Bungard machines: drill, plot and outline files
* [EH3D-GERBERVIEWER](https://github.com/JuanenRac/EH3D-GERBERVIEWER): the viewer of Gerber layers
* [EH3D-TOOL-MANAGER](https://github.com/JuanenRac/EH3D-TOOL-MANAGER): the tool crib shared by the CAM programs
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): the checking of G-code before it reaches a machine
* [EH3D-AUTOLEVEL](https://github.com/JuanenRac/EH3D-AUTOLEVEL): the height map and the correction of G-code
* [EH3D-VISION-STUDIO](https://github.com/JuanenRac/EH3D-VISION-STUDIO): cameras, fiducials and inspection

## 📚 Documentation

* [Changelog](CHANGELOG.md)
* [Architecture and structure data](docs/architecture.json): the data the two pictures are drawn from.

## 🤝 Contributing & Community

* Before a pull request run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `python -I tools/ci_validate.py`: all must pass. See [CONTRIBUTING.md](CONTRIBUTING.md) and the [code of conduct](CODE_OF_CONDUCT.md).
* The README exists in eight languages: a change to the text is made in all eight files, and the two pictures are drawn from `docs/architecture.json` and `docs/structure.json` by `tools/make_architecture.py` and `tools/make_structure.py`.
* Report a vulnerability privately: [SECURITY.md](SECURITY.md). Questions, ideas and reports: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTHOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENSE

Software: GPL-3.0-or-later. See [LICENSE](LICENSE). Hardware designs of the suite use CERN OHL-S.
