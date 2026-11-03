<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-ISOCAM banner" width="100%">
</p>

# 📖 EH3D-ISOCAM

<p align="center"><a href="README.md">🇺🇸 English</a> | 🇪🇸 <b>Español</b> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Fresado de aislamiento del cobre desde una capa Gerber: desplazamientos exactos, varias pasadas y comprobación de huecos

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.6-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Comprobación de honestidad - lo que funciona hoy:** una biblioteca Rust y un programa de línea de comandos (`eh3d-isocam`) con 184 pruebas que se ejecutan en un ordenador: traza las trayectorias de una o varias fresas alrededor del cobre de una capa Gerber, retira el cobre sobrante, elige el sentido, los puntos de inicio y el orden de los cortes, lee y escribe ficheros de taladros y contornos de placa, coloca la placa para las dos caras y para un panel, comprueba las reglas de diseño y escribe G-code o HP-GL. Todavía no tiene ventana, no se ha probado en una máquina y no se ha comprobado con una colección de ficheros reales.

---

> **Estado de seguridad:** este programa no controla ninguna máquina y no es un sistema de seguridad. Lo que produce debe revisarlo una persona antes de ejecutar nada en una máquina; nada de esto se ha ejecutado en hardware real.


## 📋 De un vistazo

| | |
|---|---|
| Versión | `0.0.6` |
| Madurez | Andamiaje |
| Pruebas | 184 |
| Pila tecnológica | Rust |
| En hardware real | No probado: nada de esto se ha ejecutado en una máquina |

**Ir a:** [Resumen](#-resumen) · [Arquitectura](#-arquitectura) · [Inicio rápido](#-inicio-rápido) · [Pruebas](#-pruebas) · [Estructura](#-estructura-del-repositorio) · [Relacionados](#-proyectos-relacionados)

## 🚦 Estado del proyecto

Cada línea es una parte de este repositorio y la luz dice hasta dónde llega hoy. Se escribe con lo que el repositorio puede demostrar (pruebas, compilaciones, ejecuciones), no con lo previsto.

🟢 Hecho y verificado · 🟡 Funciona, con límites conocidos · 🟠 Hecho a medias o sin verificar · 🔴 Sin hacer o sin funcionar

| Área | Estado | Qué significa aquí |
|---|---|---|
| Núcleo | 🟡 | traza las trayectorias de una o varias fresas alrededor del cobre de una capa Gerber, retira el cobre sobrante, elige el sentido, los puntos de inicio y el orden de los cortes, indica los huecos demasiado estrechos para la herramienta y escribe G-code o HP-GL; probado en un ordenador con entradas normales y dañadas, no con una colección de ficheros reales. |
| Ficheros y disposición | 🟡 | lee y escribe ficheros de taladros y contornos de placa, coloca la placa para las dos caras y para un panel, añade agujeros de alineación y comprueba las reglas de diseño con la posición de cada hallazgo; probado en un ordenador, no con ficheros escritos por otros programas ni en una máquina. |
| Aislamiento forzado, corrección, máscara | 🟡 | caminos forzados por los huecos donde no cabe la herramienta, trayectorias más ligeras, escala y corrección con puntos de referencia, las aberturas de la máscara de soldadura, unidades y vista previa de las pasadas; probado en un ordenador con capas generadas y entradas dañadas, nunca con placas reales ni en una máquina, y la corrección no se ha comparado con mediciones reales de una cámara. |
| Línea de comandos | 🟡 | `eh3d-isocam` ejecuta las mismas funciones que la biblioteca e imprime el resultado; cada error vuelve como mensaje y un código de salida distinto de cero. |
| Pruebas | 🟡 | 184 pruebas pasan (`cargo test`), con casos normales, casos límite y entradas dañadas. |
| Seguridad | 🟡 | Rust seguro y simple (`forbid(unsafe_code)`): nada entra en pánico con los ficheros recibidos y un valor malo es un error con su motivo; sin revisión independiente. |
| Ventana | 🔴 | Previsto: la ventana egui común a la suite; hoy solo existe la línea de comandos. |
| Hardware real | 🔴 | No probado: nada de esto se ha ejecutado en una máquina. |
| Documentación | 🟡 | README en ocho idiomas, el registro de cambios y las dos imágenes dibujadas desde `docs/`. |

## 🎯 Resumen

* **El cobre en una rejilla** (`bitmap`): pads (redondos, rectangulares, ovalados), pistas y regiones se dibujan en celdas del tamaño elegido; las pistas y pads solapados son una sola pieza porque una rejilla no tiene solapes, así que no hace falta ningún caso especial; una rejilla que no cupiera en memoria se rechaza.
* **La distancia exacta** (`eh3d-raster`): la distancia de cada celda al cobre más cercano se calcula exactamente (y se prueba contra fuerza bruta), y la trayectoria de una pasada es la línea donde vale el radio de la herramienta, trazada como polilíneas cerradas y aligerada con una tolerancia de media celda; las pasadas siguientes van un paso lateral más lejos cada una.
* **Lo que la herramienta no puede hacer** (`isolate`): donde dos piezas de cobre separadas están más cerca que el ancho de la herramienta ninguna trayectoria puede pasar entre ellas: `check` lista esos sitios y `isolate` no escribe el G-code hasta que se miren (o `--force`). El G-code (`src/gcode.rs`) levanta la herramienta entre trayectorias, baja a la profundidad elegida y corta después la trayectoria más cercana.
* **HP-GL para las máquinas Bungard** (`src/hpgl.rs`): con un fichero de salida que acaba en `.plt` las pasadas se escriben como fichero de trazado HP-GL a 40 unidades por milímetro, una plumilla por pasada, que es lo que lee el programa de fresado de las máquinas Bungard.
* **Varias herramientas y cobre sobrante** (`isolate`): un plan tiene hasta cuatro herramientas y corta primero la más ancha; donde una herramienta más ancha ya ha cortado a lo largo de una trayectoria de otra más fina, ese tramo se omite (el resto queda como tramos abiertos), y anillos de una herramienta más ancha, cada uno solapado con el anterior, pueden limpiar el cobre que queda más allá de las pasadas; el G-code se detiene para cambiar de herramienta cuando cambia su ancho. Con `around_pads_min_mm` mayor que 0, las herramientas más anchas cortan solo los anillos alrededor de pads de al menos esa anchura y la más fina hace el resto (pistas y pads pequeños).
* **Sentido, inicio y orden** (`order`): las trayectorias se trazan con el cobre a la derecha de la herramienta (fresado en concordancia con un husillo que gira en sentido horario); los ajustes pueden invertirlo (convencional), forzar horario o antihorario, empezar cada trayectoria cerrada en su punto más cercano, en su punto más bajo o donde se trazó, fijar el punto desde el que sale la herramienta y ordenar las trayectorias como se trazaron, la más cercana primero o acortando aún más el recorrido dando la vuelta a tramos del orden; `info` muestra cuántas trayectorias van en cada sentido y el recorrido en vacío.
* **Taladros y contorno de la placa** (`src/drill.rs`): los ficheros de taladros se leen (los de Bungard en milésimas de pulgada con el lector de los formatos Bungard, los métricos con el lector de la suite) y se escriben en milésimas enteras; el contorno se lee de una capa Gerber de contorno, un dibujo DXF o un fichero de trazado HP-GL, los trazos dibujados uno a uno se unen, y la trayectoria de la herramienta alrededor de la placa y dentro de sus recortes se escribe como HP-GL o como fichero Excellon de fresado. Un archivo de taladros con herramientas numeradas por encima de 255 las renumera con números libres, con un aviso, para que dos herramientas nunca se conviertan en una.
* **Dos caras, paneles y agujeros de alineación** (`transform`): la capa, los taladros y el contorno se dan la vuelta, se giran en cuartos de vuelta y se repiten en un panel a la vez; los agujeros de alineación se ponen fuera de la placa donde ésta los vuelve a encontrar una vez dada la vuelta (un tercero sobre la línea la distingue de la misma placa girada media vuelta).
* **Reglas de diseño** (`drc`): cobre más estrecho que la regla, piezas de cobre más cerca que la regla o que el ancho de la herramienta, y taladros con menos cobre alrededor que la regla, cada uno con su posición en milímetros.
* **Un fichero de ajustes** (`src/config.rs`): cada valor que se puede afinar es un ajuste con un valor por defecto, que se lee de un fichero `clave = valor` (`examples/isocam.cfg`) o se da con el mismo nombre como opción de la línea de comandos.
* **Aislamiento forzado** (`forced`): con `forced_isolation = yes` se traza además un camino por el medio de cada hueco en el que no cabe la herramienta más fina, siempre que el hueco no sea más estrecho que `forced_min_gap_mm`; cada sitio se avisa con sus coordenadas y su hueco, y los huecos más estrechos se listan y se dejan como están. Viene desactivado: entonces esos sitios solo se avisan.
* **Zonas problema, tolerancia y distancia de alineación** (`problems`, `src/config.rs`): `check` también lista, como avisos que no detienen nada, las esquinas internas más agudas que `problem_corner_angle_deg` (una herramienta redonda deja cobre en la punta de una muesca; 60 grados por defecto, 0 lo desactiva) y las pistas que pasan entre dos pads más cerca de ellas que `problem_track_gap_factor` veces la herramienta (1,5 por defecto, 0 lo desactiva); ambos criterios son ajustes, no están fijos en el código. `conflict_tolerance_mm` (0 por defecto: no se acepta ningún hueco) acepta los huecos más estrechos que la herramienta como mucho en esa cantidad, en `check` y en `isolate`. La distancia de los agujeros de alineación a la placa es `alignment_margin_mm` (5 mm por defecto; otros programas de este tipo los ponen a 10 mm, que es `alignment_margin_mm = 10`).
* **Puentes, redes, patrones de agujeros y vaciado de áreas** (`src/drill.rs`, `isolate`, `transform`, `problems`, `legacy`): `route` puede dejar puentes que sujetan la placa en la plancha (`tab_count` por cada trazado cerrado, 0 por defecto; `tab_width_mm` de material que queda; `tab_min_length_mm` entre puentes): el corte se detiene media anchura de puente y medio diámetro de herramienta antes del centro de cada puente. `isolate --net X,Y` aísla solo la red que contiene un punto, con la herramienta `net_tool_mm`. `rubout-area OUT --area X0,Y0,X1,Y1` escribe líneas paralelas de la herramienta `rubout_lines_tool_mm` (dirección `rubout_lines_direction`) que vacían un área. Los agujeros de alineación pueden ser 2, 3 o 6, fuera de la placa o, con `alignment_pattern = special`, en un patrón en torno a su centro (`alignment_special_a_mm` de separación en la pareja, `alignment_special_b_mm` para los cuatro más). Con una tolerancia de conflictos, `check` e `isolate` avisan de las pistas y pads rectangulares más finos que el doble de la tolerancia más `tolerance_warning_margin_mm`. La estimación del formato de una capa Gerber antigua usa ahora las unidades que indica el archivo y el tamaño de la placa, y una lista de aperturas puede dar los dos lados en una celda (`60x40`).
* **Tolerancia para pads, relleno por contornos, tipo de fichero, DXF y plumas del trazador** (`tolerance`, `fill`, `src/filetype.rs`, `src/dxf.rs`, `src/hpgl.rs`): `conflict_tolerance_scope = pads` (por defecto `all`, como antes) reserva la tolerancia de conflicto para los huecos que tienen un pad al lado, y el aviso de cobre fino mira entonces solo los pads. `rubout-area OUT [CAPA.gtl] --contour FICHERO` (o `--area`) rellena los trazados cerrados de un fichero, par-impar para que un trazado interior sea un agujero, con líneas de ida y vuelta de `rubout_lines_tool_mm`; con una capa, las líneas se recortan al cobre: toda la herramienta sobre cobre más `rubout_fill_margin_mm`. Una capa que en realidad es un fichero Excellon, HP-GL o DXF (o un fichero de taladros que es una capa Gerber) se rechaza por su nombre, y `kind FICHERO...` dice qué es cada fichero. `isolate` y `rubout-area` escriben un dibujo DXF si el nombre acaba en `.dxf`. `drills ENTRADA SALIDA.plt` traza los agujeros con una pluma por broca (`hpgl_pen_map = 1:2, 3:1`; las brocas que no aparecen toman las plumas libres más bajas), y `hpgl_pen_speed_cm_s` (0 = la de la máquina, por defecto) escribe la velocidad de pluma en cualquier trazado. `hpgl_isocam_style` (activado) escribe el trazado como el programa original: un `PA` por cada punto y líneas de unas 75 columnas terminadas en CR LF; desactivado escribe la forma compacta.
* **Placas de dos caras y el fichero de taladros sencillo** (`src/hpgl.rs`, `src/drill.rs`, `src/filetype.rs`): `hpgl_shift_negative = no` mantiene en el trazado las coordenadas de la placa (también las negativas), de modo que el trazado y el fichero de taladros comparten origen; `isolate` con `plot_outline = yes` y `--outline FICHERO` pone primero en el trazado el corte alrededor del contorno (con la fresa de `route_tool_mm`), con la pluma `hpgl_outline_pen`, y el aislamiento con la pluma `hpgl_isolation_pen`. `--mirror vertical` da la vuelta a los taladros respecto al centro del contorno. `ncd_isocam_style = yes` escribe el fichero de taladros como los programas de Bungard (`M48` y `%`, herramientas solo por número, coordenada repetida omitida, CR LF, `M00`) y `ncd_digits` rellena las coordenadas con ceros por delante. Un fichero de taladros sin línea `M48` se reconoce como Excellon.
* **Trayectorias más ligeras, escala y corrección** (`optimize`, `fiducial`): se quitan los picos y los puntos sobre una recta (dentro de `optimize_tolerance_mm`) pasan a ser un solo movimiento, activado por defecto (0,01 mm y 0,02 mm; con 0 se desactiva); el trabajo se puede escalar respecto al centro de la placa, y pares de puntos de referencia relativos a ese centro (dónde debería estar una marca y dónde se midió) dan una semejanza (dos pares) o un ajuste afín (tres o más, mínimos cuadrados) que se aplica a cada trayectoria y a cada taladro; se rechaza si la escala se aleja mucho de 1 o una marca no encaja.
* **Máscara de soldadura** (`mask`): las aberturas se escriben como una capa Gerber, a partir de la capa de máscara del diseño o de los pads del cobre, cada una ensanchada en `mask_margin_mm`; las pistas quedan cubiertas.
* **Unidades y vista previa** (`eh3d-raster`, `src/preview.rs`): una longitud puede llevar `mm`, `mil` o `in`, el G-code puede escribirse en pulgadas, y `preview` dibuja las pasadas sobre el cobre como dibujo SVG o como texto.
* **Capas Gerber antiguas** (`legacy`): una capa sin sentencia de formato (RS-274D, como la escriben muchos programas antiguos) se lee cuando se da su lista de aperturas (`--apertures`, un código D y su tamaño por línea) junto con su formato (`--gerber-format 2.3`, `--gerber-zeros`, `--gerber-units`); se reescribe como Gerber extendido y pasa por el lector normal. Probado con capas escritas a mano, no con ficheros de otros programas.
* **Lectura y avisos por código (línea de comandos):** los ficheros se leen con un límite de tamaño (`--max-bytes N`, 256 MiB por defecto) y el texto que no es UTF-8 se lee como Latin-1 con un aviso (`input-not-utf8`); los avisos del lector de Gerber se muestran con su código estable y `--hide CODE[,CODE]` omite esos códigos. La nota sobre los trazados más ligeros y la de los puentes del corte solo se imprimen cuando cambiaron algo.

## 🧩 Papel en EH3D

EH3D-ISOCAM es un programa de la EH3D Fabrication Suite. Hoy funciona como biblioteca y línea de comandos; su ventana, con el aspecto que comparten los demás programas de la suite, está prevista. Lee y escribe ficheros simples, así que puede usarse solo o entre los demás programas.

## 🏛️ Arquitectura

<p align="center">
  <img src="images/architecture.svg" alt="Arquitectura de EH3D-ISOCAM: quién lo usa, qué contiene y de qué depende" width="100%">
</p>


Cómo encajan las partes de este repositorio: quién lo usa, qué contiene y de qué depende. La imagen se dibuja desde `docs/architecture.json` y se comprueba con el código.

## ✨ Funciones

* **Biblioteca y línea de comandos:** cada orden de `eh3d-isocam` es una función de la biblioteca, así que otro programa de la suite puede llamarla sin lanzar un proceso.
* **Pruebas con entradas dañadas:** las pruebas también dan ficheros vacíos, basura, números enormes y ficheros cortados, y ninguna puede provocar un pánico.
* **Sin adivinar:** un valor que no tiene sentido es un error que dice cuál y por qué, nunca un resultado que solo lo parece.
* **Sin código unsafe:** `#![forbid(unsafe_code)]` en la biblioteca y el programa.
* **Cada valor afinado es un ajuste:** nada de lo que una persona pueda querer cambiar está escondido en el código; un valor malo es un error con su línea.
* **Unidades a elegir:** las longitudes de un fichero de ajustes o de la línea de comandos pueden escribirse en milímetros, mils o pulgadas; el G-code puede escribirse en pulgadas y la pantalla puede mostrar mils.

## 🔒 Seguridad y protección

| Amenaza | Defensa |
|---|---|
| Un fichero dañado, cortado o vacío | sin pánico: las pruebas cortan una entrada válida en cada posición y dan basura |
| Un número sin sentido (negativo, no finito, absurdamente grande) | se rechaza con un error que nombra el valor; nada se redondea hasta parecer plausible |
| Uso inseguro de memoria | Rust seguro y simple con `forbid(unsafe_code)` |

**Lo que no se defiende:** un resultado producido sin errores no es un resultado verificado. Nada de esto conoce los límites de una máquina y el programa no ha tenido revisión independiente. Informe de una vulnerabilidad en privado (véase [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

La biblioteca es `eh3d_isocam` (`use eh3d_isocam::...`); la línea de comandos es `eh3d-isocam`. Cada orden imprime su resultado en la salida estándar y comunica un error en la salida de errores con un código de salida distinto de cero. No hay acceso a la red. El texto de la ayuda (`eh3d-isocam help`) está en inglés:

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
options: --config FILE  --outline FILE  --drills FILE  --force  and every setting by its name (--fine-tool-mm 0.15, --fine-tool-mm 8mil, --forced-isolation yes, --fiducial "0,0,0.1,0.05")
old-style Gerber (no format statement): --apertures FILE  --gerber-format 2.3  --gerber-zeros leading|trailing  --gerber-units inch|mm  --aperture-units mil|mm|inch
```

```text
EH3D-ISOCAM/
├── src/
│   ├── config.rs                                todos los valores ajustables, desde un fichero de ajustes
│   ├── drill.rs                                 taladros y contorno de la placa, leídos y escritos
│   ├── dxf.rs                                   las pasadas como dibujo DXF
│   ├── filetype.rs                              de qué tipo es un fichero de entrada
│   ├── gcode.rs                                 los recorridos como G-code
│   ├── hpgl.rs                                  los recorridos como fichero de trazado HP-GL
│   ├── lib.rs                                   la biblioteca: lo que usa cada orden
│   ├── main.rs                                  la línea de comandos
│   └── preview.rs                               las pasadas dibujadas en SVG o texto
├── tests/
│   ├── advanced.rs                              aislamiento forzado, corrección, máscara, unidades, vista previa
│   ├── cli_reading.rs                           pruebas de lectura de ficheros: el límite de tamaño, texto que no es UTF-8, avisos por código
│   ├── files.rs                                 pruebas de los ficheros de taladros y contorno
│   ├── isocam.rs                                pruebas de la biblioteca y la línea de comandos
│   ├── kinds.rs                                 tolerancia para pads, rellenos, tipos de fichero, plumas, DXF
│   ├── layout.rs                                pruebas de doble cara, paneles, reglas y la línea de comandos
│   ├── multi.rs                                 pruebas de herramientas, cobre sobrante, sentido, orden y ajustes
│   └── readme.rs                                pruebas que ejecutan lo que prometen el README y los scripts run
├── examples/
│   ├── isocam.cfg                               todos los ajustes con su valor por defecto, para copiar y cambiar
│   └── sample.gtl                               una muestra para probar el programa
├── docs/
│   ├── architecture.json                        datos de la imagen de Arquitectura
│   └── structure.json                           datos de la imagen de Estructura
├── tools/                                       comprobación del CI y del README y herramientas de imágenes
├── images/                                      banner y las imágenes de Arquitectura y Estructura del repositorio
├── .github/                                     flujos de CI y de publicación, plantillas de issues y pull requests
├── eh3d.project.json                            manifiesto: versión, rol, madurez y comando de compilación
├── Cargo.toml Cargo.lock                        el workspace y sus dependencias fijadas
├── rust-toolchain.toml                          la toolchain de Rust fijada
├── CHANGELOG.md                                 qué cambió en cada versión
├── SECURITY.md SUPPORT.md                       cómo informar de una vulnerabilidad y dónde preguntar
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   cómo contribuir, el código de conducta y la licencia
```

## ⚙️ Configuración

Cada valor que se puede afinar es un ajuste con un valor por defecto: los diámetros, las pasadas y el paso lateral de la herramienta fina y de la gruesa, la retirada del cobre sobrante, la profundidad, el avance y la velocidad, el sentido, el punto de salida, el inicio de cada trayectoria y el orden, la herramienta que corta la placa, los agujeros de alineación, el espejo, el giro y el panel, y las reglas de diseño. `eh3d-isocam config` los imprime todos como un fichero (`examples/isocam.cfg` es ese fichero); `--config FICHERO` lee uno, y cualquier ajuste puede darse en la línea de comandos como opción con el mismo nombre (`--fine-tool-mm 0.15`), que prevalece sobre el fichero. Un valor sin sentido es un error que indica la línea. Los límites fijos viven en el código como constantes con nombre y están cubiertos por pruebas. El aislamiento forzado, las trayectorias más ligeras, la escala y los puntos de referencia, el margen de la máscara de soldadura, las unidades y el ancho de la vista previa en texto también son ajustes.

## 🚀 Inicio rápido

Necesita una cadena de herramientas Rust estable (1.88 o más reciente).

```bash
cargo build --release
cargo test
cargo run --release -- isolate examples/sample.gtl examples/sample.nc --passes 2
```

## 🧪 Pruebas

El conjunto tiene 184 pruebas (`cargo test`, ejecutadas en Windows y Linux en el CI). Comprueban los casos normales, los casos límite y las entradas dañadas: ficheros vacíos, basura y ficheros cortados nunca provocan un pánico. El CI ejecuta además `cargo fmt --check`, `cargo clippy` con las advertencias como errores y `python -I tools/ci_validate.py`.

## 🏗️ Compilación y publicación

La versión está en `Cargo.toml` y en `eh3d.project.json`; cada versión tiene su entrada en [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` compilan el programa, `test.bat` / `test.sh` ejecutan las pruebas y `run.bat` / `run.sh` lo ejecutan con un ejemplo. Las versiones se publican en GitHub.

## 📂 Estructura del repositorio

<p align="center">
  <img src="images/repository-structure.svg" alt="Estructura del repositorio EH3D-ISOCAM" width="100%">
</p>

<details>
<summary>Versión en texto del árbol</summary>

```text
EH3D-ISOCAM/
├── src/
│   ├── config.rs                                todos los valores ajustables, desde un fichero de ajustes
│   ├── drill.rs                                 taladros y contorno de la placa, leídos y escritos
│   ├── dxf.rs                                   las pasadas como dibujo DXF
│   ├── filetype.rs                              de qué tipo es un fichero de entrada
│   ├── gcode.rs                                 los recorridos como G-code
│   ├── hpgl.rs                                  los recorridos como fichero de trazado HP-GL
│   ├── lib.rs                                   la biblioteca: lo que usa cada orden
│   ├── main.rs                                  la línea de comandos
│   └── preview.rs                               las pasadas dibujadas en SVG o texto
├── tests/
│   ├── advanced.rs                              aislamiento forzado, corrección, máscara, unidades, vista previa
│   ├── cli_reading.rs                           pruebas de lectura de ficheros: el límite de tamaño, texto que no es UTF-8, avisos por código
│   ├── files.rs                                 pruebas de los ficheros de taladros y contorno
│   ├── isocam.rs                                pruebas de la biblioteca y la línea de comandos
│   ├── kinds.rs                                 tolerancia para pads, rellenos, tipos de fichero, plumas, DXF
│   ├── layout.rs                                pruebas de doble cara, paneles, reglas y la línea de comandos
│   ├── multi.rs                                 pruebas de herramientas, cobre sobrante, sentido, orden y ajustes
│   └── readme.rs                                pruebas que ejecutan lo que prometen el README y los scripts run
├── examples/
│   ├── isocam.cfg                               todos los ajustes con su valor por defecto, para copiar y cambiar
│   └── sample.gtl                               una muestra para probar el programa
├── docs/
│   ├── architecture.json                        datos de la imagen de Arquitectura
│   └── structure.json                           datos de la imagen de Estructura
├── tools/                                       comprobación del CI y del README y herramientas de imágenes
├── images/                                      banner y las imágenes de Arquitectura y Estructura del repositorio
├── .github/                                     flujos de CI y de publicación, plantillas de issues y pull requests
├── eh3d.project.json                            manifiesto: versión, rol, madurez y comando de compilación
├── Cargo.toml Cargo.lock                        el workspace y sus dependencias fijadas
├── rust-toolchain.toml                          la toolchain de Rust fijada
├── CHANGELOG.md                                 qué cambió en cada versión
├── SECURITY.md SUPPORT.md                       cómo informar de una vulnerabilidad y dónde preguntar
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   cómo contribuir, el código de conducta y la licencia
```

</details>

## 🗺️ Hoja de ruta y límites conocidos

**Límites conocidos hoy**

* Todavía no tiene ventana: solo existen la biblioteca y la línea de comandos.
* No se ha probado en una máquina ni con una colección de ficheros reales, y no ha tenido revisión independiente.
* Las trayectorias son exactas a media celda, no a la curva misma; no hay estrategia Voronoi, no se ajustan arcos, por decisión (una curva son muchos movimientos cortos), un camino forzado deja que una herramienta más ancha que el hueco toque el cobre por los dos lados, la corrección con puntos de referencia mueve las trayectorias pero conserva el tamaño del desfase de la herramienta, la escala es respecto al centro de la placa, no hay capa de serigrafía y no se ha fresado ninguna placa con estas trayectorias.
* La máscara de soldadura sigue los pads tal como están dibujados (círculos, rectángulos y óvalos); las pistas quedan cubiertas y un margen negativo no encoge una región.
* La retirada del cobre sobrante son anillos de una herramienta más ancha a cierta distancia del cobre: los sitios más estrechos que esa herramienta conservan su cobre, y no sigue el borde de la placa.
* Los tramos omitidos para una herramienta más fina se deciden en una rejilla con dos celdas de margen; prefiere cortar un tramo dos veces a dejarlo sin cortar.
* Los ficheros de taladros se escriben en milésimas de pulgada enteras (la convención de los programas Bungard): un diámetro `T1C0.8` sin unidad se lee como milímetros desde 0,2 y como pulgadas por debajo; los ficheros con ranuras o trayectorias de fresado las dejan fuera, con una nota. Los ficheros escritos solo los han releído los lectores de esta suite, no otros programas ni máquinas.
* Las reglas de diseño no redondean las esquinas de un pad cuadrado y no comprueban el cobre frente al borde de la placa ni a la máscara de soldadura; la comprobación de los anillos toma el pad tal como está dibujado en la capa Gerber.

**Siguiente**

* Añadir la ventana con el aspecto común a los programas de la suite, en los ocho idiomas, sobre la vista previa que hoy existe como texto y SVG.
* La estrategia Voronoi; comparar la corrección con puntos de referencia con mediciones de una cámara.
* Capas de serigrafía.
* Comparar las trayectorias con placas fresadas de verdad y probarlo con ficheros reales y con los demás programas de la suite.

## 🌍 Idiomas

El README y los textos existen en inglés, español, francés, italiano, alemán, chino, japonés y portugués de Portugal.

## 🔗 Proyectos relacionados

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): los lectores de Gerber, Excellon y G-code
* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): los formatos de ficheros de las máquinas Bungard: taladros, trazado y contornos
* [EH3D-GERBERVIEWER](https://github.com/JuanenRac/EH3D-GERBERVIEWER): el visor de capas Gerber
* [EH3D-TOOL-MANAGER](https://github.com/JuanenRac/EH3D-TOOL-MANAGER): el almacén de herramientas que comparten los programas CAM
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): la revisión del G-code antes de llegar a una máquina
* [EH3D-AUTOLEVEL](https://github.com/JuanenRac/EH3D-AUTOLEVEL): el mapa de alturas y la corrección del G-code
* [EH3D-VISION-STUDIO](https://github.com/JuanenRac/EH3D-VISION-STUDIO): las cámaras, los fiduciales y la inspección

## 📚 Documentación

* [Registro de cambios](CHANGELOG.md)
* [Datos de arquitectura y estructura](docs/architecture.json): los datos con los que se dibujan las dos imágenes.

## 🤝 Contribuir y comunidad

* Antes de una pull request ejecute `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` y `python -I tools/ci_validate.py`: todo debe pasar. Véanse [CONTRIBUTING.md](CONTRIBUTING.md) y el [código de conducta](CODE_OF_CONDUCT.md).
* El README existe en ocho idiomas: un cambio de texto se hace en los ocho ficheros, y las dos imágenes se dibujan desde `docs/architecture.json` y `docs/structure.json` con `tools/make_architecture.py` y `tools/make_structure.py`.
* Informe de una vulnerabilidad en privado: [SECURITY.md](SECURITY.md). Preguntas, ideas e informes: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENCIA

Software: GPL-3.0-or-later. Véase [LICENSE](LICENSE). Los diseños de hardware de la suite usan CERN OHL-S.
