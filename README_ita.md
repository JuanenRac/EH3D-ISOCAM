<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-ISOCAM banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-ISOCAM
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | 🇮🇹 <b>Italiano</b> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Fresatura di isolamento del rame da un livello Gerber: offset esatti, più passate e controllo degli spazi

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.8-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verifica di onestà - cosa funziona oggi:** una libreria Rust e un programma a riga di comando (`eh3d-isocam`) con 294 test che girano su un computer: traccia i percorsi di una o più frese attorno al rame di un livello Gerber, rimuove il rame residuo, sceglie il verso, i punti di partenza e l'ordine dei tagli, legge e scrive file di foratura e contorni della scheda, dispone la scheda per le due facce e per un pannello, controlla le regole di progetto e scrive G-code o HP-GL. Ha una finestra (compilata con `--features gui`) che apre un livello, un file di foratura, un contorno, un livello maschera e una tabella di punti di riferimento, capovolge la scheda e la ripete su un pannello, disegna il rame, i fori, il contorno e i percorsi e scrive gli stessi file della riga di comando, ma che non è ancora stata vista su uno schermo; non è stato provato su una macchina e non è stato verificato con una raccolta di file reali.

---

> **Stato di sicurezza:** questo programma non controlla alcuna macchina e non è un sistema di sicurezza. Ciò che produce deve essere controllato da una persona prima di eseguire qualsiasi cosa su una macchina; nulla di questo ha girato su hardware reale.


## 📋 In breve

| | |
|---|---|
| Versione | `0.0.8` |
| Maturità | Impalcatura |
| Test | 294 |
| Stack tecnologico | Rust |
| Su hardware reale | Non provato: nulla di questo ha girato su una macchina |

**Vai a:** [Panoramica](#-panoramica) · [Architettura](#-architettura) · [Avvio rapido](#-avvio-rapido) · [Test](#-test) · [Struttura](#-struttura-del-repository) · [Correlati](#-progetti-correlati)

## 🚦 Stato del progetto

Ogni riga è una parte di questo repository e il semaforo dice a che punto è oggi. È scritta in base a ciò che il repository può mostrare (test, compilazioni, esecuzioni), non a ciò che è previsto.

🟢 Fatto e verificato · 🟡 Funziona, con limiti noti · 🟠 Fatto a metà o non ancora verificato · 🔴 Non fatto o non funzionante

| Area | Stato | Cosa significa qui |
|---|---|---|
| Nucleo | 🟡 | traccia i percorsi di una o più frese attorno al rame di un livello Gerber, rimuove il rame residuo, sceglie il verso, i punti di partenza e l'ordine dei tagli, segnala le fessure troppo strette per l'utensile e scrive G-code o HP-GL; provato su un computer con input normali e danneggiati, non con una raccolta di file reali. |
| File e disposizione | 🟡 | legge e scrive file di foratura e contorni della scheda, dispone la scheda per le due facce e per un pannello, aggiunge fori di allineamento e controlla le regole di progetto con la posizione di ogni rilievo; provato su un computer, non con file scritti da altri programmi né su una macchina. |
| Isolamento forzato, correzione, maschera | 🟡 | percorsi forzati negli spazi in cui l'utensile non entra, percorsi più leggeri, scala e correzione con punti di riferimento, aperture della maschera di saldatura, unità e anteprima delle passate; provato su un computer con livelli generati e input danneggiati, mai con schede vere né su una macchina, e la correzione non è stata confrontata con misure reali di una telecamera. |
| Riga di comando | 🟡 | `eh3d-isocam` esegue le stesse funzioni della libreria e stampa il risultato; ogni errore torna come messaggio e codice di uscita diverso da zero. |
| Test | 🟡 | 294 test passano (`cargo test`), con casi normali, casi limite e input danneggiati. |
| Sicurezza | 🟡 | Rust sicuro e semplice (`forbid(unsafe_code)`): nulla va in panico con i file ricevuti e un valore errato è un errore con il suo motivo; nessuna revisione indipendente. |
| Finestra | 🟠 | Scritta (egui, `--features gui`): aprire o trascinare un livello, zoom e spostamento, modificare i parametri principali, esportare; la sua logica è testata, ma non è ancora stata compilata né vista su uno schermo. |
| Hardware reale | 🔴 | Non provato: nulla di questo è stato eseguito su una macchina. |
| Documentazione | 🟡 | README in otto lingue, il changelog e le due immagini disegnate da `docs/`. |

## 🎯 Panoramica

* **Il rame su una griglia** (`bitmap`): pad (tondi, rettangolari, ovali), piste e regioni sono disegnati su celle della dimensione scelta; piste e pad sovrapposti sono un unico pezzo perché una griglia non ha sovrapposizioni, quindi non serve alcun caso speciale; una griglia che non starebbe in memoria è rifiutata.
* **La distanza esatta** (`eh3d-raster`): la distanza di ogni cella dal rame più vicino è calcolata esattamente (e provata contro la forza bruta) e il percorso di una passata è la linea dove vale il raggio dell'utensile, tracciata come poligonali chiuse e alleggerita con una tolleranza di mezza cella; le passate successive vanno una passata laterale più lontano ciascuna.
* **Ciò che l'utensile non può fare** (`isolate`): dove due pezzi di rame separati sono più vicini della larghezza dell'utensile nessun percorso può passare tra loro: `check` elenca quei punti e `isolate` non scrive il G-code finché non li si guarda (o `--force`). Il G-code (`src/gcode.rs`) solleva l'utensile tra i percorsi, scende alla profondità scelta e taglia poi il percorso più vicino.
* **HP-GL per le macchine Bungard** (`src/hpgl.rs`): con un file di uscita che termina in `.plt` le passate sono scritte come file di tracciato HP-GL a 40 unità al millimetro, una penna per passata, ciò che legge il programma di fresatura delle macchine Bungard.
* **Più utensili e rame residuo** (`isolate`): un piano ha fino a quattro utensili, taglia per primo il più largo; dove un utensile più largo ha già tagliato lungo un percorso di uno più fine, quel tratto viene omesso (il resto resta come tratti aperti), e anelli di un utensile più largo, ciascuno sovrapposto al precedente, possono ripulire il rame rimasto oltre le passate; il G-code si ferma per il cambio utensile quando cambia la larghezza dell'utensile. Con `around_pads_min_mm` maggiore di 0, gli utensili più larghi tagliano solo gli anelli attorno ai pad larghi almeno così e l'utensile più sottile fa il resto (piste e pad piccoli).
* **Verso, partenza e ordine** (`order`): i percorsi sono tracciati con il rame alla destra dell'utensile (fresatura concorde con un mandrino che gira in senso orario); le impostazioni possono invertirlo (discorde), imporre orario o antiorario, far iniziare ogni percorso chiuso nel punto più vicino, nel punto più basso o dove è stato tracciato, fissare il punto da cui parte l'utensile e mettere i percorsi nell'ordine del tracciamento, il più vicino per primo o accorciato ancora girando tratti dell'ordine; `info` mostra quanti percorsi vanno in ogni verso e lo spostamento a vuoto.
* **Fori e contorno della scheda** (`src/drill.rs`): i file di foratura sono letti (quelli Bungard in millesimi di pollice con il lettore dei formati Bungard, quelli metrici con il lettore della suite) e scritti in millesimi interi; un contorno è letto da un livello Gerber di contorno, un disegno DXF o un file di tracciamento HP-GL, i tratti disegnati uno a uno vengono uniti, e il percorso dell'utensile attorno alla scheda e dentro i suoi ritagli è scritto come HP-GL o come file Excellon di fresatura. Un file di foratura con utensili numerati oltre 255 li rinumera con numeri liberi, con un avviso, così due utensili non diventano mai uno.
* **Due facce, pannelli e fori di allineamento** (`transform`): il livello, i fori e il contorno vengono ribaltati, ruotati a quarti di giro e ripetuti su un pannello insieme; i fori di allineamento sono messi fuori dalla scheda dove la scheda ribaltata li ritrova (un terzo sull'asse la distingue dalla stessa scheda ruotata di mezzo giro).
* **Regole di progetto** (`drc`): rame più stretto della regola, pezzi di rame più vicini della regola o della larghezza dell'utensile, e fori con meno rame intorno della regola, ciascuno con la sua posizione in millimetri.
* **Un file di impostazioni** (`src/config.rs`): ogni valore regolabile è un'impostazione con un valore predefinito, letta da un file `chiave = valore` (`examples/isocam.cfg`) o data con lo stesso nome come opzione della riga di comando.
* **Isolamento forzato** (`forced`): con `forced_isolation = yes` viene tracciato anche un percorso a metà di ogni spazio in cui l'utensile più fine non entra, purché lo spazio non sia più stretto di `forced_min_gap_mm`; ogni punto è segnalato con le sue coordinate e il suo spazio, e gli spazi più stretti sono elencati e lasciati stare. È disattivato per impostazione predefinita: allora quei punti sono solo segnalati.
* **Zone problematiche, tolleranza e distanza di allineamento** (`problems`, `src/config.rs`): `check` elenca anche, come avvisi che non fermano nulla, gli angoli interni più acuti di `problem_corner_angle_deg` (un utensile tondo lascia rame nella punta di una tacca; 60 gradi di default, 0 lo disattiva) e le piste che passano tra due pad più vicini ad esse di `problem_track_gap_factor` volte l'utensile (1,5 di default, 0 lo disattiva); entrambi i criteri sono impostazioni, non fissati nel codice. `conflict_tolerance_mm` (0 di default: nessuno spazio viene accettato) accetta gli spazi più stretti dell'utensile al massimo di quella quantità, in `check` e in `isolate`. La distanza dei fori di allineamento dalla scheda è `alignment_margin_mm` (5 mm di default; altri programmi di questo tipo li mettono a 10 mm, cioè `alignment_margin_mm = 10`).
* **Ponti, reti, schemi di fori e svuotamento di aree** (`src/drill.rs`, `isolate`, `transform`, `problems`, `legacy`): `route` può lasciare ponti che trattengono la scheda nella lastra (`tab_count` per ogni percorso chiuso, 0 per impostazione predefinita; `tab_width_mm` di materiale che resta; `tab_min_length_mm` tra i ponti): il taglio si ferma mezza larghezza di ponte e mezzo diametro di utensile prima del centro di ogni ponte. `isolate --net X,Y` isola solo la rete che contiene un punto, con l'utensile `net_tool_mm`. `rubout-area OUT --area X0,Y0,X1,Y1` scrive linee parallele dell'utensile `rubout_lines_tool_mm` (direzione `rubout_lines_direction`) che svuotano un'area. I fori di allineamento possono essere 2, 3 o 6, fuori dalla scheda o, con `alignment_pattern = special`, in uno schema attorno al suo centro (`alignment_special_a_mm` di distanza per la coppia, `alignment_special_b_mm` per gli altri quattro). Con una tolleranza sui conflitti, `check` e `isolate` avvisano delle piste e dei pad rettangolari più sottili del doppio della tolleranza più `tolerance_warning_margin_mm`. La stima del formato di un livello Gerber vecchio stile usa ora le unità dichiarate dal file e la dimensione della scheda, e un elenco di aperture può dare i due lati in una cella (`60x40`).
* **Tolleranza solo per i pad, riempimento per contorni, tipo di file, DXF e penne del plotter** (`tolerance`, `fill`, `src/filetype.rs`, `src/dxf.rs`, `src/hpgl.rs`): `conflict_tolerance_scope = pads` (`all` per default, come prima) riserva la tolleranza di conflitto agli spazi che hanno un pad accanto, e l'avviso di rame sottile guarda allora solo i pad. `rubout-area OUT [LIVELLO.gtl] --contour FILE` (oppure `--area`) riempie i percorsi chiusi di un file, pari-dispari così che un percorso interno sia un foro, con linee avanti e indietro di `rubout_lines_tool_mm`; con un livello le linee sono tagliate al rame: tutto l'utensile sul rame più `rubout_fill_margin_mm`. Un livello che in realtà è un file Excellon, HP-GL o DXF (o un file di foratura che è un livello Gerber) viene rifiutato per nome, e `kind FILE...` dice cos'è ogni file. `isolate` e `rubout-area` scrivono un disegno DXF per un nome in `.dxf`. `drills INGRESSO USCITA.plt` traccia i fori con una penna per punta (`hpgl_pen_map = 1:2, 3:1`; le punte non elencate prendono le penne libere più basse), e `hpgl_pen_speed_cm_s` (0 = quella della macchina, predefinito) scrive la velocità della penna in ogni tracciato. `hpgl_isocam_style` (attivo) scrive il tracciato come il programma originale: un `PA` per ogni punto e righe di circa 75 colonne terminate da CR LF; disattivato scrive la forma compatta.
* **Schede a doppia faccia e file di foratura semplice** (`src/hpgl.rs`, `src/drill.rs`, `src/filetype.rs`): `hpgl_shift_negative = no` mantiene nel tracciato le coordinate della scheda (anche quelle negative), così tracciato e file di foratura condividono l'origine; `isolate` con `plot_outline = yes` e `--outline FILE` mette per primo nel tracciato il taglio attorno al contorno (con l'utensile di `route_tool_mm`), sulla penna `hpgl_outline_pen`, con l'isolamento sulla penna `hpgl_isolation_pen`. `--mirror vertical` rovescia i fori rispetto al centro del contorno. `ncd_isocam_style = yes` scrive un file di foratura come i programmi Bungard (`M48` e `%`, utensili solo col numero, coordinata ripetuta omessa, CR LF, `M00`) e `ncd_digits` riempie le coordinate con zeri davanti. Un file di foratura senza riga `M48` è riconosciuto come Excellon. Quando i tracciati vengono spostati, `hpgl_margin_mm` fissa lo spostamento comune del tracciato di isolamento e di quello dei fori di una scheda, così restano allineati. `spindle_wait_s` scrive un'attesa (`G4`) dopo ogni avvio del mandrino nel G-code.
* **Percorsi più leggeri, scala e correzione** (`optimize`, `fiducial`): le punte sono tolte e i punti su una retta (entro `optimize_tolerance_mm`) diventano un solo movimento, attivo per impostazione predefinita (0,01 mm e 0,02 mm; con 0 si disattiva); il lavoro può essere scalato rispetto al centro della scheda, e coppie di punti di riferimento relative a quel centro (dove un segno dovrebbe essere e dove è stato misurato) danno una similitudine (due coppie) o un adattamento affine (tre o più, minimi quadrati) applicato a ogni percorso e a ogni foro; viene rifiutato se la scala è lontana da 1 o un segno non si adatta.
* **Maschera di saldatura** (`mask`): le aperture sono scritte come livello Gerber, dal livello maschera del progetto o dai pad del rame, ciascuna ingrandita di `mask_margin_mm`; le piste restano coperte.
* **Unità e anteprima** (`eh3d-raster`, `src/preview.rs`): una lunghezza può portare `mm`, `mil` o `in`, il G-code può essere scritto in pollici, e `preview` disegna le passate sul rame come disegno SVG o come testo.
* **Livelli Gerber vecchio stile** (`legacy`): un livello senza istruzione di formato (RS-274D, come lo scrivono molti programmi più vecchi) viene letto quando si indica il suo elenco di aperture (`--apertures`, un codice D e la sua dimensione per riga) insieme al suo formato (`--gerber-format 2.3`, `--gerber-zeros`, `--gerber-units`); viene riscritto come Gerber esteso e passa dal lettore normale. Provato con livelli scritti a mano, non con file di altri programmi.
* **Lettura e avvisi per codice (riga di comando):** i file sono letti con un limite di dimensione (`--max-bytes N` or `max_input_mb`, 64 MiB di default) e il testo non UTF-8 viene letto come Latin-1 con un avviso (`input-not-utf8`); gli avvisi del lettore Gerber sono mostrati con il loro codice stabile e `--hide CODE[,CODE]` omette quei codici. La nota sui percorsi alleggeriti e quella sui ponti del taglio compaiono solo quando hanno cambiato qualcosa. `arc_tolerance_mm` stabilisce con quale precisione si seguono gli archi di un livello.

## 🧩 Ruolo in EH3D

EH3D-ISOCAM è un programma della EH3D Fabrication Suite. Funziona come libreria e riga di comando e, compilato con `--features gui`, come finestra con l'aspetto comune agli altri programmi della suite. Legge e scrive file semplici, quindi può essere usato da solo o tra gli altri programmi.

## 🏛️ Architettura

<p align="center">
  <img src="images/architecture.svg" alt="Architettura di EH3D-ISOCAM: chi lo usa, che cosa contiene e da che cosa dipende" width="100%">
</p>


Come si incastrano le parti di questo repository: chi lo usa, cosa contiene e da cosa dipende. L'immagine è disegnata da `docs/architecture.json` e verificata con il codice.

## ✨ Funzionalità

* **Libreria e riga di comando:** ogni comando di `eh3d-isocam` è una funzione della libreria, così un altro programma della suite può chiamarla senza avviare un processo.
* **Test con input danneggiati:** i test forniscono anche file vuoti, spazzatura, numeri enormi e file tagliati, e nessuno può andare in panico.
* **Nessuna ipotesi:** un valore privo di senso è un errore che dice quale e perché, mai un risultato che sembra solo giusto.
* **Nessun codice unsafe:** `#![forbid(unsafe_code)]` nella libreria e nel programma.
* **Ogni valore regolato è un'impostazione:** niente di ciò che una persona può voler cambiare è nascosto nel codice; un valore sbagliato è un errore con la sua riga.
* **Unità a scelta:** le lunghezze in un file di impostazioni o sulla riga di comando possono essere scritte in millimetri, mil o pollici; il G-code può essere scritto in pollici e lo schermo può mostrare mil.
* **File più sicuri:** il G-code inizia con l'intestazione standard (generatore, unità, file di origine con il suo SHA-256), un'uscita che è anche un ingresso dell'esecuzione viene rifiutata, un tracciato con una coordinata oltre 32767 unità viene rifiutato, `--units mm|inch` dà le unità di un file di foratura che non le dice (un file con fori minuscoli e senza unità viene rifiutato senza di esso) e il contorno di tratti separati si unisce con un indice, così cinquantamila segmenti richiedono un attimo.
* **Uscita controllata e documentata:** il file dei fori dice in un commento che le sue coordinate sono millesimi di pollice interi senza punto decimale e di quanto può sbagliare l'arrotondamento; i test rileggono il G-code delle passate e quello della riga di comando con il lettore della suite, che non trova alcun avviso, alcun movimento rapido nel materiale e alcun taglio con il mandrino fermo.
* **Segni da una tabella:** `--fiducials-csv FILE` legge le coppie di segni (x di progetto, y di progetto, x misurata, y misurata; una riga di intestazione e le righe che iniziano con `#` vengono omesse), così l'uscita di `eh3d-vision align --isocam-args` si usa così com'è.

## 🔒 Sicurezza e protezione

| Minaccia | Difesa |
|---|---|
| Un file danneggiato, tagliato o vuoto | nessun panico: i test tagliano un input valido in ogni posizione e forniscono spazzatura |
| Un numero privo di senso (negativo, non finito, assurdamente grande) | rifiutato con un errore che nomina il valore; nulla viene arrotondato in qualcosa di plausibile |
| Uso non sicuro della memoria | Rust sicuro e semplice con `forbid(unsafe_code)` |

**Cosa non è difeso:** un risultato prodotto senza errori non è un risultato verificato. Nulla qui conosce i limiti di una macchina e il programma non ha avuto revisione indipendente. Segnalate una vulnerabilità in privato (vedi [SECURITY.md](SECURITY.md)).

## 🔌 Interfacce

La libreria è `eh3d_isocam` (`use eh3d_isocam::...`); la riga di comando è `eh3d-isocam`. Ogni comando stampa il risultato sullo standard output e segnala un errore sullo standard error con un codice di uscita diverso da zero. Non c'è accesso alla rete. Il testo della guida (`eh3d-isocam help`) è in inglese:

```text
eh3d-isocam isolate LAYER.gtl OUT.nc|OUT.plt|OUT.dxf  paths round the copper (G-code, HP-GL for a .plt, DXF for a .dxf)
eh3d-isocam check LAYER.gtl                      places where the tool does not fit between pieces of copper
eh3d-isocam info LAYER.gtl                       passes, paths each way round, travel through the air
eh3d-isocam preview LAYER.gtl [OUT.svg|OUT.txt]    the passes drawn over the copper, as SVG or text
eh3d-isocam drc LAYER.gtl [--drills F.ncd]       design rules, with the place of every finding
eh3d-isocam drills IN.ncd OUT.ncd|OUT.plt [--align]  read, turn over, panel and add alignment holes; .plt = a pen per drill
eh3d-isocam route OUTLINE OUT.plt|OUT.ncd        path of the tool round the board
eh3d-isocam rubout-area OUT.nc|OUT.plt|OUT.dxf [LAYER.gtl] --area X0,Y0,X1,Y1|--contour FILE  parallel lines of a tool that clear an area or fill closed paths, cut to the copper of LAYER
eh3d-isocam kind FILE...                        say what kind of file each one is (Gerber, old-style Gerber, Excellon, HP-GL, DXF)
eh3d-isocam mask COPPER.gtl OUT.gts [--mask-layer F] openings of the solder mask, from the mask layer or the pads
eh3d-isocam config                               every setting with its default, as a file
eh3d-isocam [gui] [LAYER.gtl]                    the window (a build with --features gui); with no arguments it opens
options: --config FILE  --outline FILE  --drills FILE  --force  and every setting by its name (--fine-tool-mm 0.15, --fine-tool-mm 8mil, --forced-isolation yes, --fiducial "0,0,0.1,0.05")
old-style Gerber (no format statement): --apertures FILE  --gerber-format 2.3  --gerber-zeros leading|trailing  --gerber-units inch|mm  --aperture-units mil|mm|inch
```

```text
EH3D-ISOCAM/
├── src/
│   ├── board.rs                                 i passi condivisi da riga di comando e finestra
│   ├── browse.rs                                l'elenco dei file della finestra che apre un file
│   ├── config.rs                                tutti i valori regolabili, da un file di impostazioni
│   ├── draw.rs                                  ciò che disegna la finestra: le forme e i triangoli delle regioni
│   ├── drill.rs                                 fori e contorno della scheda, letti e scritti
│   ├── dxf.rs                                   le passate come disegno DXF
│   ├── filetype.rs                              di che tipo è un file di ingresso
│   ├── gcode.rs                                 i percorsi come G-code
│   ├── gui.rs                                   la finestra (egui, con la funzione gui): disegna lo stato e inoltra ciò che fa la persona
│   ├── hpgl.rs                                  i percorsi come file di tracciamento HP-GL
│   ├── lang.rs                                  le parole della finestra nelle otto lingue
│   ├── lib.rs                                   la libreria: ciò che usa ogni comando
│   ├── main.rs                                  la riga di comando
│   ├── preview.rs                               le passate disegnate in SVG o testo
│   ├── viewport.rs                              zoom e spostamento dell'immagine, in semplici numeri
│   └── window.rs                                lo stato della finestra senza schermo: livello, parametri, percorsi, esportazione e stato
├── tests/
│   ├── advanced.rs                              isolamento forzato, correzione, maschera, unità, anteprima
│   ├── cli_reading.rs                           test di lettura dei file: il limite di dimensione, testo non UTF-8, avvisi per codice
│   ├── files.rs                                 test dei file di foratura e contorno
│   ├── isocam.rs                                test della libreria e della riga di comando
│   ├── kinds.rs                                 tolleranza per pad, riempimenti, tipi di file, penne, DXF
│   ├── layout.rs                                test di doppia faccia, pannelli, regole e riga di comando
│   ├── multi.rs                                 test di utensili, rame residuo, verso, ordine e impostazioni
│   ├── problems.rs                              test delle zone problematiche, della tolleranza e dei fori di allineamento
│   ├── readme.rs                                test che eseguono ciò che promettono il README e gli script run
│   └── window_cli.rs                            test che una compilazione senza la funzione gui resti una riga di comando
├── examples/
│   ├── isocam.cfg                               tutte le impostazioni con il valore predefinito, da copiare e modificare
│   └── sample.gtl                               un esempio per provare il programma
├── docs/
│   ├── architecture.json                        dati dell'immagine di Architettura
│   └── structure.json                           dati dell'immagine di Struttura
├── tools/                                       controllo della CI e del README e strumenti per le immagini
├── images/                                      banner e le immagini di Architettura e Struttura del repository
├── .github/                                     workflow di CI e di rilascio, modelli di issue e pull request
├── eh3d.project.json                            manifesto: versione, ruolo, maturità e comando di compilazione
├── Cargo.toml Cargo.lock                        il workspace e le sue dipendenze bloccate
├── rust-toolchain.toml                          la toolchain Rust fissata
├── CHANGELOG.md                                 cosa è cambiato in ogni versione
├── SECURITY.md SUPPORT.md                       come segnalare una vulnerabilità e dove chiedere
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   come contribuire, codice di condotta e licenza
```

## ⚙️ Configurazione

Ogni valore regolabile è un'impostazione con un valore predefinito: diametri, passate e passo laterale dell'utensile fine e di quello grosso, la rimozione del rame residuo, la profondità, l'avanzamento e la velocità, il verso, il punto di partenza, l'inizio di ogni percorso e l'ordine, l'utensile che taglia la scheda, i fori di allineamento, lo specchio, la rotazione e il pannello, e le regole di progetto. `eh3d-isocam config` le stampa tutte come file (`examples/isocam.cfg` è quel file); `--config FILE` ne legge uno, e ogni impostazione può essere data sulla riga di comando come opzione con lo stesso nome (`--fine-tool-mm 0.15`), che prevale sul file. Un valore privo di senso è un errore che indica la riga. I limiti fissi vivono nel codice come costanti con nome e sono coperti da test. Anche l'isolamento forzato, i percorsi più leggeri, la scala e i punti di riferimento, il margine della maschera di saldatura, le unità e la larghezza dell'anteprima in testo sono impostazioni.

## 🚀 Avvio rapido

Serve una toolchain Rust stabile (1.88 o più recente).

```bash
cargo build --release
cargo test
cargo run --release -- isolate examples/sample.gtl examples/sample.nc --passes 2
cargo run --release --features gui   # the window
```

## 🧪 Test

La suite ha 294 test (`cargo test`, eseguiti su Windows e Linux nella CI). Verificano i casi normali, i casi limite e gli input danneggiati: file vuoti, spazzatura e file tagliati non causano mai un panico. La CI esegue anche `cargo fmt --check`, `cargo clippy` con gli avvisi come errori e `python -I tools/ci_validate.py`.

## 🏗️ Compilazione e rilascio

La versione sta in `Cargo.toml` e in `eh3d.project.json`; ogni versione ha la sua voce in [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` compilano il programma, `test.bat` / `test.sh` eseguono i test e `run.bat` / `run.sh` lo eseguono con un esempio. Le versioni sono pubblicate su GitHub.

## 📂 Struttura del repository

<p align="center">
  <img src="images/repository-structure.svg" alt="Struttura del repository EH3D-ISOCAM" width="100%">
</p>

<details>
<summary>Versione testuale dell'albero</summary>

```text
EH3D-ISOCAM/
├── src/
│   ├── board.rs                                 i passi condivisi da riga di comando e finestra
│   ├── browse.rs                                l'elenco dei file della finestra che apre un file
│   ├── config.rs                                tutti i valori regolabili, da un file di impostazioni
│   ├── draw.rs                                  ciò che disegna la finestra: le forme e i triangoli delle regioni
│   ├── drill.rs                                 fori e contorno della scheda, letti e scritti
│   ├── dxf.rs                                   le passate come disegno DXF
│   ├── filetype.rs                              di che tipo è un file di ingresso
│   ├── gcode.rs                                 i percorsi come G-code
│   ├── gui.rs                                   la finestra (egui, con la funzione gui): disegna lo stato e inoltra ciò che fa la persona
│   ├── hpgl.rs                                  i percorsi come file di tracciamento HP-GL
│   ├── lang.rs                                  le parole della finestra nelle otto lingue
│   ├── lib.rs                                   la libreria: ciò che usa ogni comando
│   ├── main.rs                                  la riga di comando
│   ├── preview.rs                               le passate disegnate in SVG o testo
│   ├── viewport.rs                              zoom e spostamento dell'immagine, in semplici numeri
│   └── window.rs                                lo stato della finestra senza schermo: livello, parametri, percorsi, esportazione e stato
├── tests/
│   ├── advanced.rs                              isolamento forzato, correzione, maschera, unità, anteprima
│   ├── cli_reading.rs                           test di lettura dei file: il limite di dimensione, testo non UTF-8, avvisi per codice
│   ├── files.rs                                 test dei file di foratura e contorno
│   ├── isocam.rs                                test della libreria e della riga di comando
│   ├── kinds.rs                                 tolleranza per pad, riempimenti, tipi di file, penne, DXF
│   ├── layout.rs                                test di doppia faccia, pannelli, regole e riga di comando
│   ├── multi.rs                                 test di utensili, rame residuo, verso, ordine e impostazioni
│   ├── problems.rs                              test delle zone problematiche, della tolleranza e dei fori di allineamento
│   ├── readme.rs                                test che eseguono ciò che promettono il README e gli script run
│   └── window_cli.rs                            test che una compilazione senza la funzione gui resti una riga di comando
├── examples/
│   ├── isocam.cfg                               tutte le impostazioni con il valore predefinito, da copiare e modificare
│   └── sample.gtl                               un esempio per provare il programma
├── docs/
│   ├── architecture.json                        dati dell'immagine di Architettura
│   └── structure.json                           dati dell'immagine di Struttura
├── tools/                                       controllo della CI e del README e strumenti per le immagini
├── images/                                      banner e le immagini di Architettura e Struttura del repository
├── .github/                                     workflow di CI e di rilascio, modelli di issue e pull request
├── eh3d.project.json                            manifesto: versione, ruolo, maturità e comando di compilazione
├── Cargo.toml Cargo.lock                        il workspace e le sue dipendenze bloccate
├── rust-toolchain.toml                          la toolchain Rust fissata
├── CHANGELOG.md                                 cosa è cambiato in ogni versione
├── SECURITY.md SUPPORT.md                       come segnalare una vulnerabilità e dove chiedere
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   come contribuire, codice di condotta e licenza
```

</details>

## 🗺️ Roadmap e limiti noti

**Limiti noti oggi**

* La finestra non è stata vista su uno schermo; usa le impostazioni predefinite oltre ai parametri che mostra e non copre i comandi `check`, `drc`, `info`, `preview` e `rubout-area`: restano nella riga di comando.
* Non è stato provato né su una macchina né con una raccolta di file reali, e non ha avuto alcuna revisione indipendente.
* I percorsi sono esatti a mezza cella, non alla curva stessa; non c'è una strategia Voronoi, gli archi non sono adattati, per scelta (una curva è fatta di molti piccoli movimenti), un percorso forzato lascia che un utensile più largo dello spazio tocchi il rame da entrambi i lati, la correzione con punti di riferimento sposta i percorsi ma mantiene la dimensione dello scostamento dell'utensile, la scala è rispetto al centro della scheda, non c'è un livello di serigrafia e nessuna scheda è stata fresata con questi percorsi.
* La maschera di saldatura segue i pad come disegnati (cerchi, rettangoli e ovali); le piste restano coperte e un margine negativo non rimpicciolisce una regione.
* La rimozione del rame residuo avviene con anelli di un utensile più largo a distanza dal rame: i punti più stretti di quell'utensile conservano il loro rame, e non segue il bordo della scheda.
* I tratti omessi per un utensile più fine sono decisi su una griglia con due celle di margine; si preferisce tagliare un tratto due volte che lasciarlo non tagliato.
* I file di foratura sono scritti in millesimi di pollice interi (convenzione dei programmi Bungard): un diametro `T1C0.8` senza unità è letto come millimetri da 0,2 in su e come pollici al di sotto; i file con asole o percorsi di fresatura li lasciano fuori con una nota. I file scritti sono stati riletti solo dai lettori di questa suite, non da altri programmi o macchine.
* Le regole di progetto non arrotondano gli angoli di un pad quadrato e non controllano il rame rispetto al bordo della scheda né alla solder mask; il controllo degli anelli prende il pad come disegnato nel livello Gerber.

**Prossimi passi**

* Vedere la finestra su uno schermo e portarvi i controlli della riga di comando (`check`, `drc`, l'anteprima).
* La strategia Voronoi; confrontare la correzione con punti di riferimento con misure di una telecamera.
* Livelli di serigrafia.
* Confrontare i percorsi con schede realmente fresate e provarlo con file reali e con gli altri programmi della suite.

## 🌍 Lingue

Il README e i testi esistono in inglese, spagnolo, francese, italiano, tedesco, cinese, giapponese e portoghese europeo.

## 🔗 Progetti correlati

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): i lettori di Gerber, Excellon e G-code
* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): i formati di file delle macchine Bungard: foratura, tracciamento e contorni
* [EH3D-GERBERVIEWER](https://github.com/JuanenRac/EH3D-GERBERVIEWER): il visualizzatore dei livelli Gerber
* [EH3D-TOOL-MANAGER](https://github.com/JuanenRac/EH3D-TOOL-MANAGER): il magazzino utensili condiviso dai programmi CAM
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): il controllo del G-code prima che arrivi a una macchina
* [EH3D-AUTOLEVEL](https://github.com/JuanenRac/EH3D-AUTOLEVEL): la mappa delle altezze e la correzione del G-code
* [EH3D-VISION-STUDIO](https://github.com/JuanenRac/EH3D-VISION-STUDIO): le telecamere, i fiduciali e l'ispezione

## 📚 Documentazione

* [Changelog](CHANGELOG.md)
* [Dati di architettura e struttura](docs/architecture.json): i dati da cui si disegnano le due immagini.

## 🤝 Contribuire e comunità

* Prima di una pull request eseguite `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` e `python -I tools/ci_validate.py`: tutto deve passare. Vedi [CONTRIBUTING.md](CONTRIBUTING.md) e il [codice di condotta](CODE_OF_CONDUCT.md).
* Il README esiste in otto lingue: una modifica al testo si fa in tutti e otto i file, e le due immagini sono disegnate da `docs/architecture.json` e `docs/structure.json` con `tools/make_architecture.py` e `tools/make_structure.py`.
* Segnalate una vulnerabilità in privato: [SECURITY.md](SECURITY.md). Domande, idee e segnalazioni: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTORE

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENZA

Software: GPL-3.0-or-later. Vedi [LICENSE](LICENSE). I progetti hardware della suite usano la CERN OHL-S.
