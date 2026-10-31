<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-ISOCAM banner" width="100%">
</p>

# 📖 EH3D-ISOCAM

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | 🇮🇹 <b>Italiano</b> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Fresatura di isolamento del rame da un livello Gerber: offset esatti, più passate e controllo degli spazi

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.2-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verifica di onestà - cosa funziona oggi:** una libreria Rust e un programma a riga di comando (`eh3d-isocam`) con 60 test che girano su un computer: traccia i percorsi di una o più frese attorno al rame di un livello Gerber, rimuove il rame residuo, sceglie il verso, i punti di partenza e l'ordine dei tagli, legge e scrive file di foratura e contorni della scheda, dispone la scheda per le due facce e per un pannello, controlla le regole di progetto e scrive G-code o HP-GL. Non ha ancora una finestra, non è stato provato su una macchina e non è stato verificato con una raccolta di file reali.

---

> **Stato di sicurezza:** questo programma non controlla alcuna macchina e non è un sistema di sicurezza. Ciò che produce deve essere controllato da una persona prima di eseguire qualsiasi cosa su una macchina; nulla di questo ha girato su hardware reale.


## 📋 In breve

| | |
|---|---|
| Versione | `0.0.2` |
| Maturità | Impalcatura |
| Test | 60 |
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
| Riga di comando | 🟡 | `eh3d-isocam` esegue le stesse funzioni della libreria e stampa il risultato; ogni errore torna come messaggio e codice di uscita diverso da zero. |
| Test | 🟡 | 60 test passano (`cargo test`), con casi normali, casi limite e input danneggiati. |
| Sicurezza | 🟡 | Rust sicuro e semplice (`forbid(unsafe_code)`): nulla va in panico con i file ricevuti e un valore errato è un errore con il suo motivo; nessuna revisione indipendente. |
| Finestra | 🔴 | Previsto: la finestra egui comune alla suite; oggi esiste solo la riga di comando. |
| Hardware reale | 🔴 | Non provato: nulla di questo è stato eseguito su una macchina. |
| Documentazione | 🟡 | README in otto lingue, il changelog e le due immagini disegnate da `docs/`. |

## 🎯 Panoramica

* **Il rame su una griglia** (`src/bitmap.rs`): pad (tondi, rettangolari, ovali), piste e regioni sono disegnati su celle della dimensione scelta; piste e pad sovrapposti sono un unico pezzo perché una griglia non ha sovrapposizioni, quindi non serve alcun caso speciale; una griglia che non starebbe in memoria è rifiutata.
* **La distanza esatta** (`src/edt.rs`, `src/contour.rs`): la distanza di ogni cella dal rame più vicino è calcolata esattamente (e provata contro la forza bruta) e il percorso di una passata è la linea dove vale il raggio dell'utensile, tracciata come poligonali chiuse e alleggerita con una tolleranza di mezza cella; le passate successive vanno una passata laterale più lontano ciascuna.
* **Ciò che l'utensile non può fare** (`src/isolate.rs`): dove due pezzi di rame separati sono più vicini della larghezza dell'utensile nessun percorso può passare tra loro: `check` elenca quei punti e `isolate` non scrive il G-code finché non li si guarda (o `--force`). Il G-code (`src/gcode.rs`) solleva l'utensile tra i percorsi, scende alla profondità scelta e taglia poi il percorso più vicino.
* **HP-GL per le macchine Bungard** (`src/hpgl.rs`): con un file di uscita che termina in `.plt` le passate sono scritte come file di tracciato HP-GL a 40 unità al millimetro, una penna per passata, ciò che legge il programma di fresatura delle macchine Bungard.
* **Più utensili e rame residuo** (`src/isolate.rs`): un piano ha fino a quattro utensili, taglia per primo il più largo; dove un utensile più largo ha già tagliato lungo un percorso di uno più fine, quel tratto viene omesso (il resto resta come tratti aperti), e anelli di un utensile più largo, ciascuno sovrapposto al precedente, possono ripulire il rame rimasto oltre le passate; il G-code si ferma per il cambio utensile quando cambia la larghezza dell'utensile.
* **Verso, partenza e ordine** (`src/order.rs`): i percorsi sono tracciati con il rame alla destra dell'utensile (fresatura concorde con un mandrino che gira in senso orario); le impostazioni possono invertirlo (discorde), imporre orario o antiorario, far iniziare ogni percorso chiuso nel punto più vicino, nel punto più basso o dove è stato tracciato, fissare il punto da cui parte l'utensile e mettere i percorsi nell'ordine del tracciamento, il più vicino per primo o accorciato ancora girando tratti dell'ordine; `info` mostra quanti percorsi vanno in ogni verso e lo spostamento a vuoto.
* **Fori e contorno della scheda** (`src/drill.rs`): i file di foratura sono letti (quelli Bungard in millesimi di pollice con il lettore dei formati Bungard, quelli metrici con il lettore della suite) e scritti in millesimi interi; un contorno è letto da un livello Gerber di contorno, un disegno DXF o un file di tracciamento HP-GL, i tratti disegnati uno a uno vengono uniti, e il percorso dell'utensile attorno alla scheda e dentro i suoi ritagli è scritto come HP-GL o come file Excellon di fresatura.
* **Due facce, pannelli e fori di allineamento** (`src/transform.rs`): il livello, i fori e il contorno vengono ribaltati, ruotati a quarti di giro e ripetuti su un pannello insieme; i fori di allineamento sono messi fuori dalla scheda dove la scheda ribaltata li ritrova (un terzo sull'asse la distingue dalla stessa scheda ruotata di mezzo giro).
* **Regole di progetto** (`src/drc.rs`): rame più stretto della regola, pezzi di rame più vicini della regola o della larghezza dell'utensile, e fori con meno rame intorno della regola, ciascuno con la sua posizione in millimetri.
* **Un file di impostazioni** (`src/config.rs`): ogni valore regolabile è un'impostazione con un valore predefinito, letta da un file `chiave = valore` (`examples/isocam.cfg`) o data con lo stesso nome come opzione della riga di comando.

## 🧩 Ruolo in EH3D

EH3D-ISOCAM è un programma della EH3D Fabrication Suite. Oggi funziona come libreria e riga di comando; la sua finestra, con l'aspetto comune agli altri programmi della suite, è prevista. Legge e scrive file semplici, quindi può essere usato da solo o tra gli altri programmi.

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
│   ├── bitmap.rs                                il rame su una griglia
│   ├── config.rs                                tutti i valori regolabili, da un file di impostazioni
│   ├── contour.rs                               i percorsi di una passata, tracciati e alleggeriti
│   ├── drc.rs                                   regole di progetto: larghezze, distanze e anelli, con le posizioni
│   ├── drill.rs                                 fori e contorno della scheda, letti e scritti
│   ├── edt.rs                                   la distanza esatta dal rame
│   ├── gcode.rs                                 i percorsi come G-code
│   ├── hpgl.rs                                  i percorsi come file di tracciamento HP-GL
│   ├── isolate.rs                               utensili, passate, rame residuo e fessure strette
│   ├── lib.rs                                   la libreria: ciò che usa ogni comando
│   ├── main.rs                                  la riga di comando
│   ├── order.rs                                 verso, punti di partenza e ordine dei tagli
│   └── transform.rs                             doppia faccia, pannelli e fori di allineamento
├── tests/
│   ├── files.rs                                 test dei file di foratura e contorno
│   ├── isocam.rs                                test della libreria e della riga di comando
│   ├── layout.rs                                test di doppia faccia, pannelli, regole e riga di comando
│   └── multi.rs                                 test di utensili, rame residuo, verso, ordine e impostazioni
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
├── CHANGELOG.md                                 cosa è cambiato in ogni versione
├── SECURITY.md SUPPORT.md                       come segnalare una vulnerabilità e dove chiedere
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   come contribuire, codice di condotta e licenza
```

## ⚙️ Configurazione

Ogni valore regolabile è un'impostazione con un valore predefinito: diametri, passate e passo laterale dell'utensile fine e di quello grosso, la rimozione del rame residuo, la profondità, l'avanzamento e la velocità, il verso, il punto di partenza, l'inizio di ogni percorso e l'ordine, l'utensile che taglia la scheda, i fori di allineamento, lo specchio, la rotazione e il pannello, e le regole di progetto. `eh3d-isocam config` le stampa tutte come file (`examples/isocam.cfg` è quel file); `--config FILE` ne legge uno, e ogni impostazione può essere data sulla riga di comando come opzione con lo stesso nome (`--fine-tool-mm 0.15`), che prevale sul file. Un valore privo di senso è un errore che indica la riga. I limiti fissi vivono nel codice come costanti con nome e sono coperti da test.

## 🚀 Avvio rapido

Serve una toolchain Rust stabile (1.88 o più recente).

```bash
cargo build --release
cargo test
cargo run --release -- isolate examples/sample.gtl examples/sample.nc --passes 2
```

## 🧪 Test

La suite ha 60 test (`cargo test`, eseguiti su Windows e Linux nella CI). Verificano i casi normali, i casi limite e gli input danneggiati: file vuoti, spazzatura e file tagliati non causano mai un panico. La CI esegue anche `cargo fmt --check`, `cargo clippy` con gli avvisi come errori e `python -I tools/ci_validate.py`.

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
│   ├── bitmap.rs                                il rame su una griglia
│   ├── config.rs                                tutti i valori regolabili, da un file di impostazioni
│   ├── contour.rs                               i percorsi di una passata, tracciati e alleggeriti
│   ├── drc.rs                                   regole di progetto: larghezze, distanze e anelli, con le posizioni
│   ├── drill.rs                                 fori e contorno della scheda, letti e scritti
│   ├── edt.rs                                   la distanza esatta dal rame
│   ├── gcode.rs                                 i percorsi come G-code
│   ├── hpgl.rs                                  i percorsi come file di tracciamento HP-GL
│   ├── isolate.rs                               utensili, passate, rame residuo e fessure strette
│   ├── lib.rs                                   la libreria: ciò che usa ogni comando
│   ├── main.rs                                  la riga di comando
│   ├── order.rs                                 verso, punti di partenza e ordine dei tagli
│   └── transform.rs                             doppia faccia, pannelli e fori di allineamento
├── tests/
│   ├── files.rs                                 test dei file di foratura e contorno
│   ├── isocam.rs                                test della libreria e della riga di comando
│   ├── layout.rs                                test di doppia faccia, pannelli, regole e riga di comando
│   └── multi.rs                                 test di utensili, rame residuo, verso, ordine e impostazioni
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
├── CHANGELOG.md                                 cosa è cambiato in ogni versione
├── SECURITY.md SUPPORT.md                       come segnalare una vulnerabilità e dove chiedere
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   come contribuire, codice di condotta e licenza
```

</details>

## 🗺️ Roadmap e limiti noti

**Limiti noti oggi**

* Non ha ancora una finestra: esistono solo la libreria e la riga di comando.
* Non è stato provato né su una macchina né con una raccolta di file reali, e non ha avuto alcuna revisione indipendente.
* I percorsi sono esatti a mezza cella, non alla curva stessa; non c'è la strategia di Voronoi, non c'è isolamento forzato dove l'utensile non entra (quei punti sono solo segnalati), non c'è scalatura né compensazione della scheda, non ci sono livelli di solder mask né di serigrafia, e con questi percorsi non è stata fresata nessuna scheda.
* La rimozione del rame residuo avviene con anelli di un utensile più largo a distanza dal rame: i punti più stretti di quell'utensile conservano il loro rame, e non segue il bordo della scheda.
* I tratti omessi per un utensile più fine sono decisi su una griglia con due celle di margine; si preferisce tagliare un tratto due volte che lasciarlo non tagliato.
* I file di foratura sono scritti in millesimi di pollice interi (convenzione dei programmi Bungard): un diametro `T1C0.8` senza unità è letto come millimetri da 0,2 in su e come pollici al di sotto; i file con asole o percorsi di fresatura li lasciano fuori con una nota. I file scritti sono stati riletti solo dai lettori di questa suite, non da altri programmi o macchine.
* Le regole di progetto non arrotondano gli angoli di un pad quadrato e non controllano il rame rispetto al bordo della scheda né alla solder mask; il controllo degli anelli prende il pad come disegnato nel livello Gerber.

**Prossimi passi**

* Aggiungere la finestra con l'aspetto comune ai programmi della suite, nelle otto lingue, con anteprima delle passate e unità in mil, millimetri e pollici.
* Isolamento forzato dove l'utensile non entra (una passata più stretta per quel tratto, con avviso) e la strategia di Voronoi; ripulire gli archi; scalatura e compensazione della scheda dai fiduciali della telecamera.
* Livelli di solder mask e serigrafia.
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
