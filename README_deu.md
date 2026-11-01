<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-ISOCAM banner" width="100%">
</p>

# 📖 EH3D-ISOCAM

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | 🇩🇪 <b>Deutsch</b> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Isolationsfräsen von Kupfer aus einer Gerber-Lage: exakte Offsets, mehrere Durchgänge und Lückenprüfung

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.3-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Ehrlichkeitscheck - was heute wirklich läuft:** eine Rust-Bibliothek und ein Kommandozeilenprogramm (`eh3d-isocam`) mit 134 Tests, die auf einem Computer laufen: zeichnet die Bahnen eines oder mehrerer Fräswerkzeuge um das Kupfer einer Gerber-Lage, räumt das Restkupfer ab, wählt Richtung, Startpunkte und Reihenfolge der Schnitte, liest und schreibt Bohrdateien und Platinenkonturen, richtet eine Platine für beide Seiten und für einen Nutzen aus, prüft die Entwurfsregeln und schreibt G-Code oder HP-GL. Es hat noch kein Fenster, wurde auf keiner Maschine erprobt und nicht an einer Sammlung echter Dateien geprüft.

---

> **Sicherheitsstatus:** Dieses Programm steuert keine Maschine und ist kein Sicherheitssystem. Was es erzeugt, muss ein Mensch prüfen, bevor etwas an einer Maschine läuft; nichts davon lief auf echter Hardware.


## 📋 Auf einen Blick

| | |
|---|---|
| Version | `0.0.3` |
| Reifegrad | Gerüst |
| Tests | 134 |
| Technologie-Stack | Rust |
| Auf echter Hardware | Nicht erprobt: nichts davon lief an einer Maschine |

**Springe zu:** [Überblick](#-überblick) · [Architektur](#-architektur) · [Schnellstart](#-schnellstart) · [Tests](#-tests) · [Struktur](#-struktur-des-repositorys) · [Verwandte](#-verwandte-projekte)

## 🚦 Projektstatus

Jede Zeile ist ein Teil dieses Repositorys, und die Ampel sagt, wie weit er heute ist. Sie beruht auf dem, was das Repository zeigen kann (Tests, Builds, Läufe), nicht auf dem, was geplant ist.

🟢 Fertig und überprüft · 🟡 Funktioniert, mit bekannten Grenzen · 🟠 Teilweise fertig oder noch nicht überprüft · 🔴 Nicht gemacht oder funktioniert nicht

| Bereich | Stand | Was das hier bedeutet |
|---|---|---|
| Kern | 🟡 | zeichnet die Bahnen eines oder mehrerer Fräswerkzeuge um das Kupfer einer Gerber-Lage, räumt das Restkupfer ab, wählt Richtung, Startpunkte und Reihenfolge der Schnitte, meldet die für das Werkzeug zu engen Lücken und schreibt G-Code oder HP-GL; auf einem Computer mit normalen und beschädigten Eingaben getestet, nicht mit einer Sammlung echter Dateien. |
| Dateien und Anordnung | 🟡 | liest und schreibt Bohrdateien und Platinenkonturen, richtet eine Platine für beide Seiten und für einen Nutzen aus, setzt Passlöcher und prüft die Entwurfsregeln mit dem Ort jedes Befunds; auf einem Computer getestet, nicht mit Dateien anderer Programme und nicht auf einer Maschine. |
| Erzwungene Isolation, Korrektur, Maske | 🟡 | erzwungene Pfade durch Lücken, in die das Werkzeug nicht passt, leichtere Bahnen, Skalierung und Korrektur mit Referenzpunkten, die Öffnungen der Lötstoppmaske, Einheiten und die Vorschau der Durchgänge; auf einem Computer mit erzeugten Lagen und beschädigten Eingaben getestet, nie mit echten Platinen und nie auf einer Maschine, und die Korrektur wurde nicht mit echten Kameramessungen verglichen. |
| Kommandozeile | 🟡 | `eh3d-isocam` führt dieselben Funktionen wie die Bibliothek aus und gibt das Ergebnis aus; jeder Fehler kommt als Meldung mit einem Exit-Code ungleich null zurück. |
| Tests | 🟡 | 134 Tests bestehen (`cargo test`), mit Normalfällen, Grenzfällen und beschädigten Eingaben. |
| Sicherheit | 🟡 | Reines sicheres Rust (`forbid(unsafe_code)`): nichts gerät bei den übergebenen Dateien in Panik und ein schlechter Wert ist ein Fehler mit Begründung; keine unabhängige Prüfung. |
| Fenster | 🔴 | Geplant: das egui-Fenster, das die Suite gemeinsam nutzt; heute gibt es nur die Kommandozeile. |
| Echte Hardware | 🔴 | Nicht erprobt: nichts davon wurde an einer Maschine ausgeführt. |
| Dokumentation | 🟡 | README in acht Sprachen, das Changelog und die zwei aus `docs/` gezeichneten Bilder. |

## 🎯 Überblick

* **Das Kupfer auf einem Raster** (`src/bitmap.rs`): Pads (rund, rechteckig, oval), Leiterbahnen und Flächen werden auf Zellen der gewählten Größe gezeichnet; überlappende Bahnen und Pads sind ein Stück, weil ein Raster keine Überlappungen kennt, es braucht also keinen Sonderfall; ein Raster, das nicht in den Speicher passt, wird abgelehnt.
* **Der exakte Abstand** (`src/edt.rs`, `src/contour.rs`): Der Abstand jeder Zelle zum nächsten Kupfer wird exakt berechnet (und gegen Brute Force getestet), und die Bahn eines Durchgangs ist die Linie, auf der er dem Werkzeugradius entspricht, als geschlossene Polylinien verfolgt und mit einer Toleranz von einer halben Zelle ausgedünnt; weitere Durchgänge liegen je eine Zustellung weiter außen.
* **Was das Werkzeug nicht kann** (`src/isolate.rs`): Wo zwei getrennte Kupferstücke näher beieinander liegen als das Werkzeug breit ist, passt keine Bahn dazwischen: `check` listet diese Stellen, und `isolate` schreibt den G-Code erst, wenn man sie angesehen hat (oder `--force`). Der G-Code (`src/gcode.rs`) hebt das Werkzeug zwischen den Bahnen an, fährt auf die gewählte Tiefe und schneidet als Nächstes die nächstgelegene Bahn.
* **HP-GL für die Bungard-Maschinen** (`src/hpgl.rs`): Bei einer Ausgabedatei mit der Endung `.plt` werden die Durchgänge als HP-GL-Plotdatei mit 40 Einheiten je Millimeter geschrieben, ein Stift je Durchgang; das liest das Fräsprogramm der Bungard-Maschinen.
* **Mehrere Werkzeuge und Restkupfer** (`src/isolate.rs`): ein Plan hat bis zu vier Werkzeuge, das breiteste schneidet zuerst; wo ein breiteres Werkzeug schon entlang einer Bahn eines schmaleren geschnitten hat, entfällt dieses Stück (der Rest bleibt als offene Abschnitte), und Ringe eines breiteren Werkzeugs, die sich jeweils mit dem vorigen überlappen, können das Kupfer jenseits der Durchgänge abräumen; der G-Code hält zum Werkzeugwechsel an, wenn sich die Werkzeugbreite ändert. Ist `around_pads_min_mm` größer als 0, schneiden die breiteren Werkzeuge nur die Ringe um Pads mit mindestens dieser Breite, und das feinste Werkzeug übernimmt den Rest (Leiterbahnen und kleine Pads).
* **Richtung, Start und Reihenfolge** (`src/order.rs`): die Bahnen werden mit dem Kupfer rechts vom Werkzeug gezeichnet (Gleichlauffräsen bei rechtsdrehender Spindel); die Einstellungen können das umkehren (Gegenlauf), im oder gegen den Uhrzeigersinn erzwingen, jede geschlossene Bahn an ihrem nächsten Punkt, ihrem tiefsten Punkt oder dort beginnen lassen, wo sie gezeichnet wurde, den Startpunkt des Werkzeugs festlegen und die Bahnen in der gezeichneten Reihenfolge, die nächste zuerst oder durch Umkehren von Abschnitten der Reihenfolge noch kürzer ordnen; `info` zeigt, wie viele Bahnen in welche Richtung laufen, und den Leerweg.
* **Bohrungen und Platinenkontur** (`src/drill.rs`): Bohrdateien werden gelesen (Bungard-Dateien in Mil mit dem Leser der Bungard-Formate, metrische mit dem Leser der Suite) und in ganzen Mil geschrieben; eine Kontur wird aus einer Gerber-Konturlage, einer DXF-Zeichnung oder einer HP-GL-Plotdatei gelesen, einzeln gezeichnete Striche werden verbunden, und die Bahn des Werkzeugs um die Platine und in ihren Ausschnitten wird als HP-GL oder als Excellon-Fräsdatei geschrieben.
* **Beide Seiten, Nutzen und Passlöcher** (`src/transform.rs`): Lage, Bohrungen und Kontur werden zusammen gewendet, um Vierteldrehungen gedreht und auf einem Nutzen wiederholt; Passlöcher liegen außerhalb der Platine dort, wo die gewendete Platine sie wiederfindet (ein drittes auf der Achse unterscheidet sie von derselben, um eine halbe Drehung gedrehten Platine).
* **Entwurfsregeln** (`src/drc.rs`): Kupfer schmaler als die Regel, Kupferstücke näher als die Regel oder als das Werkzeug breit ist, und Bohrungen mit weniger Kupfer ringsum als die Regel, jeweils mit ihrem Ort in Millimetern.
* **Eine Einstellungsdatei** (`src/config.rs`): jeder einstellbare Wert ist eine Einstellung mit Standardwert, gelesen aus einer `Schlüssel = Wert`-Datei (`examples/isocam.cfg`) oder unter demselben Namen als Option der Kommandozeile angegeben.
* **Erzwungene Isolation** (`src/forced.rs`): mit `forced_isolation = yes` wird zusätzlich ein Pfad durch die Mitte jeder Lücke gelegt, in die das feinste Werkzeug nicht passt, solange die Lücke nicht schmaler als `forced_min_gap_mm` ist; jede Stelle wird mit Koordinaten und Lückenbreite gemeldet, schmalere Lücken werden aufgelistet und bleiben unberührt. Standardmäßig ist sie aus: Dann werden diese Stellen nur gemeldet.
* **Problemzonen, Toleranz und Passlochabstand** (`src/problems.rs`, `src/config.rs`): `check` listet außerdem als Warnungen, die nichts anhalten, die Innenecken auf, die spitzer sind als `problem_corner_angle_deg` (ein rundes Werkzeug lässt in der Spitze einer Kerbe Kupfer stehen; standardmäßig 60 Grad, 0 schaltet es ab), und die Leiterbahnen, die zwischen zwei Pads verlaufen, die ihr näher sind als `problem_track_gap_factor` mal das Werkzeug (standardmäßig 1,5, 0 schaltet es ab); beide Kriterien sind Einstellungen und nicht im Code festgelegt. `conflict_tolerance_mm` (standardmäßig 0: keine Lücke wird akzeptiert) akzeptiert die Lücken, die höchstens um diesen Betrag schmaler sind als das Werkzeug, in `check` und in `isolate`. Der Abstand der Passlöcher zur Platine ist `alignment_margin_mm` (standardmäßig 5 mm; andere Programme dieser Art setzen sie 10 mm entfernt, also `alignment_margin_mm = 10`).
* **Stege, Netze, Lochmuster und Flächenräumung** (`src/drill.rs`, `src/isolate.rs`, `src/transform.rs`, `src/problems.rs`, `src/legacy.rs`): `route` kann Stege stehen lassen, die die Platine im Nutzen halten (`tab_count` je geschlossenem Pfad, standardmäßig 0; `tab_width_mm` verbleibendes Material; `tab_min_length_mm` zwischen den Stegen): der Schnitt endet eine halbe Stegbreite und einen halben Werkzeugdurchmesser vor der Mitte jedes Stegs. `isolate --net X,Y` isoliert nur das Netz, das einen Punkt enthält, mit dem Werkzeug `net_tool_mm`. `rubout-area OUT --area X0,Y0,X1,Y1` schreibt parallele Linien des Werkzeugs `rubout_lines_tool_mm` (Richtung `rubout_lines_direction`), die eine Fläche räumen. Die Passlöcher können 2, 3 oder 6 sein, außerhalb der Platine oder mit `alignment_pattern = special` in einem Muster um ihre Mitte (`alignment_special_a_mm` Abstand beim Paar, `alignment_special_b_mm` für die vier weiteren). Mit einer Konflikttoleranz warnen `check` und `isolate` vor Leiterbahnen und rechteckigen Pads, die dünner sind als die doppelte Toleranz plus `tolerance_warning_margin_mm`. Die Schätzung des Formats einer alten Gerber-Ebene nutzt jetzt die Einheiten, die die Datei nennt, und die Größe der Platine, und eine Aperturliste darf beide Seiten in einer Zelle angeben (`60x40`).
* **Leichtere Bahnen, Skalierung und Korrektur** (`src/optimize.rs`, `src/fiducial.rs`): Spitzen werden entfernt und Punkte auf einer Geraden (innerhalb von `optimize_tolerance_mm`) werden zu einer Bewegung, standardmäßig aktiv (0,01 mm und 0,02 mm; mit 0 abgeschaltet); die Arbeit lässt sich um die Mitte der Platine skalieren, und Paare von Referenzpunkten relativ zu dieser Mitte (wo eine Marke sein sollte und wo sie gemessen wurde) ergeben eine Ähnlichkeit (zwei Paare) oder eine affine Anpassung (drei oder mehr, kleinste Quadrate), die auf jede Bahn und jede Bohrung angewendet wird; abgelehnt wird sie, wenn der Maßstab weit von 1 abweicht oder eine Marke nicht passt.
* **Lötstoppmaske** (`src/mask.rs`): die Öffnungen werden als Gerber-Lage geschrieben, aus der Maskenlage des Entwurfs oder aus den Pads des Kupfers, jede um `mask_margin_mm` vergrößert; Leiterbahnen bleiben bedeckt.
* **Einheiten und Vorschau** (`src/units.rs`, `src/preview.rs`): eine Länge kann `mm`, `mil` oder `in` tragen, der G-Code kann in Zoll geschrieben werden, und `preview` zeichnet die Durchgänge über das Kupfer als SVG-Zeichnung oder als Text.
* **Alte Gerber-Ebenen** (`src/legacy.rs`): eine Ebene ohne Formatangabe (RS-274D, wie viele ältere Programme sie schreiben) wird gelesen, wenn ihre Aperturliste (`--apertures`, ein D-Code und seine Größe pro Zeile) zusammen mit ihrem Format (`--gerber-format 2.3`, `--gerber-zeros`, `--gerber-units`) angegeben wird; sie wird als erweitertes Gerber neu geschrieben und läuft durch den normalen Leser. Getestet mit von Hand geschriebenen Ebenen, nicht mit Dateien anderer Programme.

## 🧩 Rolle in EH3D

EH3D-ISOCAM ist ein Programm der EH3D Fabrication Suite. Heute arbeitet es als Bibliothek und Kommandozeile; sein Fenster im gemeinsamen Aussehen der Suite ist geplant. Es liest und schreibt einfache Dateien und lässt sich allein oder zwischen den anderen Programmen nutzen.

## 🏛️ Architektur

<p align="center">
  <img src="images/architecture.svg" alt="Architektur von EH3D-ISOCAM: wer es nutzt, was es enthält und wovon es abhängt" width="100%">
</p>


Wie die Teile dieses Repositorys zusammenpassen: wer es nutzt, was es enthält und wovon es abhängt. Das Bild wird aus `docs/architecture.json` gezeichnet und gegen den Code geprüft.

## ✨ Funktionen

* **Bibliothek und Kommandozeile:** Jeder Befehl von `eh3d-isocam` ist eine Funktion der Bibliothek, sodass ein anderes Programm der Suite ihn ohne Prozessstart aufrufen kann.
* **Tests mit beschädigten Eingaben:** Die Tests füttern auch leere Dateien, Datenmüll, riesige Zahlen und abgeschnittene Dateien; keiner darf in Panik geraten.
* **Kein Raten:** Ein unsinniger Wert ist ein Fehler, der sagt welcher und warum, nie ein Ergebnis, das nur richtig aussieht.
* **Kein unsafe-Code:** `#![forbid(unsafe_code)]` in Bibliothek und Programm.
* **Jeder abgestimmte Wert ist eine Einstellung:** nichts, was jemand ändern möchte, steckt fest im Code; ein schlechter Wert ist ein Fehler mit Zeilenangabe.
* **Einheiten nach Wahl:** Längen in einer Einstellungsdatei oder auf der Kommandozeile können in Millimetern, Mil oder Zoll geschrieben werden; der G-Code kann in Zoll geschrieben werden und der Bildschirm kann Mil anzeigen.

## 🔒 Sicherheit und Schutz

| Bedrohung | Schutz |
|---|---|
| Eine beschädigte, abgeschnittene oder leere Datei | keine Panik: die Tests schneiden gültige Eingaben an jeder Stelle ab und füttern Datenmüll |
| Eine unsinnige Zahl (negativ, nicht endlich, absurd groß) | wird mit einem Fehler abgelehnt, der den Wert nennt; nichts wird zu etwas Plausiblem gerundet |
| Unsichere Speichernutzung | reines sicheres Rust mit `forbid(unsafe_code)` |

**Was nicht abgesichert ist:** Ein fehlerfrei erzeugtes Ergebnis ist kein geprüftes Ergebnis. Nichts hier kennt die Grenzen einer Maschine, und das Programm hatte keine unabhängige Prüfung. Eine Schwachstelle bitte privat melden (siehe [SECURITY.md](SECURITY.md)).

## 🔌 Schnittstellen

Die Bibliothek ist `eh3d_isocam` (`use eh3d_isocam::...`); die Kommandozeile ist `eh3d-isocam`. Jeder Befehl gibt sein Ergebnis auf der Standardausgabe aus und meldet einen Fehler auf der Fehlerausgabe mit einem Exit-Code ungleich null. Es gibt keinen Netzwerkzugriff. Der Hilfetext (`eh3d-isocam help`) ist englisch:

```text
eh3d-isocam isolate LAYER.gtl OUT.nc|OUT.plt     paths round the copper (G-code, or HP-GL for a .plt)
eh3d-isocam check LAYER.gtl                      places where the tool does not fit between pieces of copper
eh3d-isocam info LAYER.gtl                       passes, paths each way round, travel through the air
eh3d-isocam preview LAYER.gtl [OUT.svg|OUT.txt]    the passes drawn over the copper, as SVG or text
eh3d-isocam drc LAYER.gtl [--drills F.ncd]       design rules, with the place of every finding
eh3d-isocam drills IN.ncd OUT.ncd [--align]      read, turn over, panel and add alignment holes
eh3d-isocam route OUTLINE OUT.plt|OUT.ncd        path of the tool round the board
eh3d-isocam rubout-area OUT.nc|OUT.plt --area X0,Y0,X1,Y1  parallel lines of a tool that clear an area
eh3d-isocam mask COPPER.gtl OUT.gts [--mask-layer F] openings of the solder mask, from the mask layer or the pads
eh3d-isocam config                               every setting with its default, as a file
options: --config FILE  --outline FILE  --drills FILE  --force  and every setting by its name (--fine-tool-mm 0.15, --fine-tool-mm 8mil, --forced-isolation yes, --fiducial "0,0,0.1,0.05")
old-style Gerber (no format statement): --apertures FILE  --gerber-format 2.3  --gerber-zeros leading|trailing  --gerber-units inch|mm  --aperture-units mil|mm|inch
```

```text
EH3D-ISOCAM/
├── src/
│   ├── bitmap.rs                                das Kupfer auf einem Raster
│   ├── config.rs                                alle einstellbaren Werte aus einer Einstellungsdatei
│   ├── contour.rs                               die Bahnen eines Durchgangs, verfolgt und vereinfacht
│   ├── drc.rs                                   Entwurfsregeln: Breiten, Abstände und Ringe, mit Orten
│   ├── drill.rs                                 Bohrungen und Platinenkontur lesen und schreiben
│   ├── edt.rs                                   der genaue Abstand zum Kupfer
│   ├── fiducial.rs                              Skalierung und Korrektur mit Referenzpunkten
│   ├── forced.rs                                Pfade durch Lücken, in die das Werkzeug nicht passt
│   ├── gcode.rs                                 die Bahnen als G-Code
│   ├── hpgl.rs                                  die Bahnen als HP-GL-Plotdatei
│   ├── isolate.rs                               Werkzeuge, Durchgänge, Restkupfer und enge Lücken
│   ├── legacy.rs                                alte Gerber-Ebenen: Format und Aperturliste
│   ├── lib.rs                                   die Bibliothek: was jeder Befehl nutzt
│   ├── main.rs                                  die Kommandozeile
│   ├── mask.rs                                  die Öffnungen der Lötstoppmaske
│   ├── optimize.rs                              leichtere Bahnen: Spitzen und gerade Punkte
│   ├── order.rs                                 Richtung, Startpunkte und Reihenfolge der Schnitte
│   ├── preview.rs                               die Durchgänge als SVG oder Text
│   ├── problems.rs                              Problemzonen: spitze Innenecken und Leiterbahnen zwischen Pads
│   ├── transform.rs                             beide Seiten, Nutzen und Passlöcher
│   └── units.rs                                 mm, mil und Zoll
├── tests/
│   ├── advanced.rs                              erzwungene Isolation, Korrektur, Maske, Einheiten, Vorschau
│   ├── files.rs                                 Tests der Bohr- und Konturdateien
│   ├── isocam.rs                                Tests von Bibliothek und Kommandozeile
│   ├── layout.rs                                Tests zu beiden Seiten, Nutzen, Regeln und der Kommandozeile
│   ├── legacy.rs                                Tests alter Gerber-Ebenen und der Kommandozeile
│   ├── multi.rs                                 Tests zu Werkzeugen, Restkupfer, Richtung, Reihenfolge und Einstellungen
│   └── problems.rs                              Tests der Problemzonen, der Toleranz und der Passlöcher
├── examples/
│   ├── isocam.cfg                               alle Einstellungen mit ihrem Standardwert, zum Kopieren und Ändern
│   └── sample.gtl                               ein Beispiel zum Ausprobieren
├── docs/
│   ├── architecture.json                        Daten des Architektur-Bildes
│   └── structure.json                           Daten des Struktur-Bildes
├── tools/                                       CI-Prüfung, README-Prüfung und die Bildwerkzeuge
├── images/                                      Banner und die Bilder von Architektur und Repository-Struktur
├── .github/                                     CI- und Release-Workflows, Vorlagen für Issues und Pull Requests
├── eh3d.project.json                            Manifest: Version, Rolle, Reifegrad und Build-Befehl
├── Cargo.toml Cargo.lock                        der Workspace und seine festgelegten Abhängigkeiten
├── CHANGELOG.md                                 was sich in jeder Version geändert hat
├── SECURITY.md SUPPORT.md                       Schwachstellen melden und Fragen stellen
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   Mitwirken, Verhaltenskodex und Lizenz
```

## ⚙️ Konfiguration

Jeder einstellbare Wert ist eine Einstellung mit Standardwert: Durchmesser, Durchgänge und Zustellung des feinen und des groben Werkzeugs, das Abräumen des Restkupfers, Tiefe, Vorschub und Drehzahl, die Richtung, der Startpunkt, der Beginn jeder Bahn und die Reihenfolge, das Werkzeug, das die Platine ausschneidet, die Passlöcher, Spiegelung, Drehung und Nutzen sowie die Entwurfsregeln. `eh3d-isocam config` gibt sie alle als Datei aus (`examples/isocam.cfg` ist diese Datei); `--config DATEI` liest eine, und jede Einstellung kann in der Kommandozeile als Option mit demselben Namen angegeben werden (`--fine-tool-mm 0.15`), die der Datei vorgeht. Ein unsinniger Wert ist ein Fehler, der die Zeile nennt. Die festen Grenzen stehen als benannte Konstanten im Code und sind durch Tests abgedeckt. Auch die erzwungene Isolation, die leichteren Bahnen, die Skalierung und die Referenzpunkte, der Rand der Lötstoppmaske, die Einheiten und die Breite der Textvorschau sind Einstellungen.

## 🚀 Schnellstart

Sie brauchen eine stabile Rust-Toolchain (1.88 oder neuer).

```bash
cargo build --release
cargo test
cargo run --release -- isolate examples/sample.gtl examples/sample.nc --passes 2
```

## 🧪 Tests

Die Suite hat 134 Tests (`cargo test`, in der CI unter Windows und Linux). Sie prüfen Normalfälle, Grenzfälle und beschädigte Eingaben: leere Dateien, Datenmüll und abgeschnittene Dateien lösen nie eine Panik aus. Die CI führt außerdem `cargo fmt --check`, `cargo clippy` mit Warnungen als Fehlern und `python -I tools/ci_validate.py` aus.

## 🏗️ Build und Veröffentlichung

Die Version steht in `Cargo.toml` und in `eh3d.project.json`; jede Version hat ihren Eintrag in [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` bauen das Programm, `test.bat` / `test.sh` führen die Tests aus und `run.bat` / `run.sh` starten es mit einem Beispiel. Releases erscheinen auf GitHub.

## 📂 Struktur des Repositorys

<p align="center">
  <img src="images/repository-structure.svg" alt="Struktur des Repositorys EH3D-ISOCAM" width="100%">
</p>

<details>
<summary>Textversion des Baums</summary>

```text
EH3D-ISOCAM/
├── src/
│   ├── bitmap.rs                                das Kupfer auf einem Raster
│   ├── config.rs                                alle einstellbaren Werte aus einer Einstellungsdatei
│   ├── contour.rs                               die Bahnen eines Durchgangs, verfolgt und vereinfacht
│   ├── drc.rs                                   Entwurfsregeln: Breiten, Abstände und Ringe, mit Orten
│   ├── drill.rs                                 Bohrungen und Platinenkontur lesen und schreiben
│   ├── edt.rs                                   der genaue Abstand zum Kupfer
│   ├── fiducial.rs                              Skalierung und Korrektur mit Referenzpunkten
│   ├── forced.rs                                Pfade durch Lücken, in die das Werkzeug nicht passt
│   ├── gcode.rs                                 die Bahnen als G-Code
│   ├── hpgl.rs                                  die Bahnen als HP-GL-Plotdatei
│   ├── isolate.rs                               Werkzeuge, Durchgänge, Restkupfer und enge Lücken
│   ├── legacy.rs                                alte Gerber-Ebenen: Format und Aperturliste
│   ├── lib.rs                                   die Bibliothek: was jeder Befehl nutzt
│   ├── main.rs                                  die Kommandozeile
│   ├── mask.rs                                  die Öffnungen der Lötstoppmaske
│   ├── optimize.rs                              leichtere Bahnen: Spitzen und gerade Punkte
│   ├── order.rs                                 Richtung, Startpunkte und Reihenfolge der Schnitte
│   ├── preview.rs                               die Durchgänge als SVG oder Text
│   ├── problems.rs                              Problemzonen: spitze Innenecken und Leiterbahnen zwischen Pads
│   ├── transform.rs                             beide Seiten, Nutzen und Passlöcher
│   └── units.rs                                 mm, mil und Zoll
├── tests/
│   ├── advanced.rs                              erzwungene Isolation, Korrektur, Maske, Einheiten, Vorschau
│   ├── files.rs                                 Tests der Bohr- und Konturdateien
│   ├── isocam.rs                                Tests von Bibliothek und Kommandozeile
│   ├── layout.rs                                Tests zu beiden Seiten, Nutzen, Regeln und der Kommandozeile
│   ├── legacy.rs                                Tests alter Gerber-Ebenen und der Kommandozeile
│   ├── multi.rs                                 Tests zu Werkzeugen, Restkupfer, Richtung, Reihenfolge und Einstellungen
│   └── problems.rs                              Tests der Problemzonen, der Toleranz und der Passlöcher
├── examples/
│   ├── isocam.cfg                               alle Einstellungen mit ihrem Standardwert, zum Kopieren und Ändern
│   └── sample.gtl                               ein Beispiel zum Ausprobieren
├── docs/
│   ├── architecture.json                        Daten des Architektur-Bildes
│   └── structure.json                           Daten des Struktur-Bildes
├── tools/                                       CI-Prüfung, README-Prüfung und die Bildwerkzeuge
├── images/                                      Banner und die Bilder von Architektur und Repository-Struktur
├── .github/                                     CI- und Release-Workflows, Vorlagen für Issues und Pull Requests
├── eh3d.project.json                            Manifest: Version, Rolle, Reifegrad und Build-Befehl
├── Cargo.toml Cargo.lock                        der Workspace und seine festgelegten Abhängigkeiten
├── CHANGELOG.md                                 was sich in jeder Version geändert hat
├── SECURITY.md SUPPORT.md                       Schwachstellen melden und Fragen stellen
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   Mitwirken, Verhaltenskodex und Lizenz
```

</details>

## 🗺️ Roadmap und bekannte Grenzen

**Bekannte Grenzen heute**

* Es hat noch kein Fenster: es gibt nur die Bibliothek und die Kommandozeile.
* Es wurde weder auf einer Maschine noch mit einer Sammlung echter Dateien erprobt und hatte keine unabhängige Prüfung.
* Die Bahnen sind auf eine halbe Zelle genau, nicht auf die Kurve selbst; es gibt keine Voronoi-Strategie, Bögen werden bewusst nicht angepasst (eine Kurve sind viele kurze Bewegungen), ein erzwungener Pfad lässt ein Werkzeug, das breiter als die Lücke ist, das Kupfer auf beiden Seiten berühren, die Korrektur mit Referenzpunkten verschiebt die Bahnen, behält aber die Größe des Werkzeugabstands, die Skalierung geht von der Mitte der Platine aus, es gibt keine Bestückungsdruck-Lage, und mit diesen Bahnen wurde keine Platine gefräst.
* Die Lötstoppmaske folgt den Pads wie gezeichnet (Kreise, Rechtecke und Langlöcher); Leiterbahnen bleiben bedeckt, und ein negativer Rand verkleinert keine Fläche.
* Das Abräumen des Restkupfers sind Ringe eines breiteren Werkzeugs im Abstand vom Kupfer: Stellen, die schmaler als dieses Werkzeug sind, behalten ihr Kupfer, und es folgt nicht dem Rand der Platine.
* Die für ein schmaleres Werkzeug ausgelassenen Stücke werden auf einem Raster mit zwei Zellen Spielraum entschieden; lieber wird ein Stück doppelt geschnitten als ungeschnitten gelassen.
* Bohrdateien werden in ganzen Mil geschrieben (Konvention der Bungard-Programme): ein Durchmesser `T1C0.8` ohne Einheit wird ab 0,2 als Millimeter und darunter als Zoll gelesen; Dateien mit Langlöchern oder gefrästen Bahnen lassen diese mit einem Hinweis weg. Die geschriebenen Dateien wurden nur von den Lesern dieser Suite zurückgelesen, nicht von anderen Programmen oder Maschinen.
* Die Entwurfsregeln runden die Ecken eines eckigen Pads nicht ab und prüfen das Kupfer weder gegen den Platinenrand noch gegen die Lötstoppmaske; die Prüfung der Ringe nimmt das Pad, wie es in der Gerber-Lage gezeichnet ist.

**Als Nächstes**

* Das Fenster im gemeinsamen Aussehen der Programme der Suite ergänzen, in den acht Sprachen, auf der Vorschau, die es heute als Text und SVG gibt.
* Die Voronoi-Strategie; die Korrektur mit Referenzpunkten mit Kameramessungen vergleichen.
* Bestückungsdruck-Lagen.
* Die Bahnen mit wirklich gefrästen Platinen vergleichen und mit echten Dateien und den anderen Programmen der Suite erproben.

## 🌍 Sprachen

README und Texte gibt es auf Englisch, Spanisch, Französisch, Italienisch, Deutsch, Chinesisch, Japanisch und europäischem Portugiesisch.

## 🔗 Verwandte Projekte

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): die Leser für Gerber, Excellon und G-Code
* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): die Dateiformate der Bungard-Maschinen: Bohr-, Plot- und Konturdateien
* [EH3D-GERBERVIEWER](https://github.com/JuanenRac/EH3D-GERBERVIEWER): der Betrachter für Gerber-Lagen
* [EH3D-TOOL-MANAGER](https://github.com/JuanenRac/EH3D-TOOL-MANAGER): das Werkzeuglager der CAM-Programme
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): die Prüfung von G-Code, bevor er eine Maschine erreicht
* [EH3D-AUTOLEVEL](https://github.com/JuanenRac/EH3D-AUTOLEVEL): die Höhenkarte und die Korrektur von G-Code
* [EH3D-VISION-STUDIO](https://github.com/JuanenRac/EH3D-VISION-STUDIO): Kameras, Passmarken und Inspektion

## 📚 Dokumentation

* [Changelog](CHANGELOG.md)
* [Architektur- und Strukturdaten](docs/architecture.json): die Daten, aus denen die beiden Bilder gezeichnet werden.

## 🤝 Mitwirken und Community

* Vor einem Pull Request `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` und `python -I tools/ci_validate.py` ausführen: alles muss bestehen. Siehe [CONTRIBUTING.md](CONTRIBUTING.md) und den [Verhaltenskodex](CODE_OF_CONDUCT.md).
* Die README gibt es in acht Sprachen: Eine Textänderung erfolgt in allen acht Dateien, und die zwei Bilder werden aus `docs/architecture.json` und `docs/structure.json` mit `tools/make_architecture.py` und `tools/make_structure.py` gezeichnet.
* Eine Schwachstelle privat melden: [SECURITY.md](SECURITY.md). Fragen, Ideen und Berichte: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LIZENZ

Software: GPL-3.0-or-later. Siehe [LICENSE](LICENSE). Die Hardware-Entwürfe der Suite verwenden die CERN OHL-S.
