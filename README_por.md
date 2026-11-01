<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-ISOCAM banner" width="100%">
</p>

# 📖 EH3D-ISOCAM

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | 🇵🇹 <b>Português</b></p>

### Fresagem de isolamento do cobre a partir de uma camada Gerber: deslocamentos exatos, várias passagens e verificação de espaços

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.3-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verificação de honestidade - o que funciona hoje:** uma biblioteca Rust e um programa de linha de comandos (`eh3d-isocam`) com 107 testes que correm num computador: traça os percursos de uma ou mais fresas à volta do cobre de uma camada Gerber, retira o cobre restante, escolhe o sentido, os pontos de partida e a ordem dos cortes, lê e escreve ficheiros de furação e contornos da placa, dispõe a placa para as duas faces e para um painel, verifica as regras de projeto e escreve G-code ou HP-GL. Ainda não tem janela, não foi experimentado numa máquina e não foi verificado com uma coleção de ficheiros reais.

---

> **Estado de segurança:** este programa não controla nenhuma máquina e não é um sistema de segurança. O que produz deve ser verificado por uma pessoa antes de se executar algo numa máquina; nada disto correu em hardware real.


## 📋 Num relance

| | |
|---|---|
| Versão | `0.0.3` |
| Maturidade | Estrutura inicial |
| Testes | 107 |
| Tecnologias | Rust |
| Em hardware real | Não experimentado: nada disto correu numa máquina |

**Ir para:** [Visão geral](#-visão-geral) · [Arquitetura](#-arquitetura) · [Início rápido](#-início-rápido) · [Testes](#-testes) · [Estrutura](#-estrutura-do-repositório) · [Relacionados](#-projetos-relacionados)

## 🚦 Estado do projeto

Cada linha é uma parte deste repositório e a luz diz até onde chega hoje. É escrita com o que o repositório consegue demonstrar (testes, compilações, execuções), não com o que está previsto.

🟢 Feito e verificado · 🟡 Funciona, com limites conhecidos · 🟠 Feito a meio ou ainda por verificar · 🔴 Por fazer ou a não funcionar

| Área | Estado | O que significa aqui |
|---|---|---|
| Núcleo | 🟡 | traça os percursos de uma ou mais fresas à volta do cobre de uma camada Gerber, retira o cobre restante, escolhe o sentido, os pontos de partida e a ordem dos cortes, indica as folgas demasiado estreitas para a ferramenta e escreve G-code ou HP-GL; testado num computador com entradas normais e danificadas, não com uma coleção de ficheiros reais. |
| Ficheiros e disposição | 🟡 | lê e escreve ficheiros de furação e contornos da placa, dispõe a placa para as duas faces e para um painel, acrescenta furos de alinhamento e verifica as regras de projeto com a posição de cada achado; testado num computador, não com ficheiros escritos por outros programas nem numa máquina. |
| Isolamento forçado, correção, máscara | 🟡 | percursos forçados nos espaços onde a ferramenta não cabe, percursos mais leves, escala e correção com pontos de referência, aberturas da máscara de soldadura, unidades e pré-visualização das passagens; testado num computador com camadas geradas e entradas danificadas, nunca com placas reais nem numa máquina, e a correção não foi comparada com medições reais de uma câmara. |
| Linha de comandos | 🟡 | `eh3d-isocam` executa as mesmas funções da biblioteca e imprime o resultado; cada erro volta como mensagem e código de saída diferente de zero. |
| Testes | 🟡 | 107 testes passam (`cargo test`), com casos normais, casos-limite e entradas danificadas. |
| Segurança | 🟡 | Rust seguro e simples (`forbid(unsafe_code)`): nada entra em pânico com os ficheiros recebidos e um valor mau é um erro com o seu motivo; sem revisão independente. |
| Janela | 🔴 | Previsto: a janela egui comum à suíte; hoje existe apenas a linha de comandos. |
| Hardware real | 🔴 | Não experimentado: nada disto foi executado numa máquina. |
| Documentação | 🟡 | README em oito idiomas, o registo de alterações e as duas imagens desenhadas a partir de `docs/`. |

## 🎯 Visão geral

* **O cobre numa grelha** (`src/bitmap.rs`): pads (redondos, retangulares, ovais), pistas e regiões são desenhados em células do tamanho escolhido; pistas e pads sobrepostos são uma só peça porque uma grelha não tem sobreposições, por isso não é preciso nenhum caso especial; uma grelha que não coubesse na memória é recusada.
* **A distância exata** (`src/edt.rs`, `src/contour.rs`): a distância de cada célula ao cobre mais próximo calcula-se exatamente (e testa-se contra força bruta), e a trajetória de uma passagem é a linha onde vale o raio da ferramenta, traçada como polilinhas fechadas e aliviada com uma tolerância de meia célula; as passagens seguintes vão cada uma um passo lateral mais longe.
* **O que a ferramenta não consegue fazer** (`src/isolate.rs`): onde duas peças de cobre separadas estão mais perto do que a largura da ferramenta nenhuma trajetória passa entre elas: `check` lista esses sítios e `isolate` não escreve o G-code até serem vistos (ou `--force`). O G-code (`src/gcode.rs`) levanta a ferramenta entre trajetórias, desce à profundidade escolhida e corta a seguir a trajetória mais próxima.
* **HP-GL para as máquinas Bungard** (`src/hpgl.rs`): com um ficheiro de saída que acaba em `.plt` as passagens são escritas como ficheiro de traçado HP-GL a 40 unidades por milímetro, uma caneta por passagem, que é o que lê o programa de fresagem das máquinas Bungard.
* **Várias ferramentas e cobre restante** (`src/isolate.rs`): um plano tem até quatro ferramentas e corta primeiro a mais larga; onde uma ferramenta mais larga já cortou ao longo de um percurso de outra mais fina, esse troço é omitido (o resto fica como troços abertos), e anéis de uma ferramenta mais larga, cada um sobreposto ao anterior, podem limpar o cobre que fica para além das passagens; o G-code pára para a troca de ferramenta quando a largura da ferramenta muda.
* **Sentido, início e ordem** (`src/order.rs`): os percursos são traçados com o cobre à direita da ferramenta (fresagem em concordância com um fuso que gira no sentido horário); as definições podem invertê-lo (convencional), forçar horário ou anti-horário, começar cada percurso fechado no ponto mais próximo, no ponto mais baixo ou onde foi traçado, fixar o ponto de onde a ferramenta parte e ordenar os percursos pela ordem do traçado, o mais próximo primeiro ou encurtado ainda mais virando troços da ordem; `info` mostra quantos percursos vão em cada sentido e o deslocamento em vazio.
* **Furos e contorno da placa** (`src/drill.rs`): os ficheiros de furação são lidos (os da Bungard em milésimos de polegada com o leitor dos formatos Bungard, os métricos com o leitor da suite) e escritos em milésimos inteiros; um contorno é lido de uma camada Gerber de contorno, de um desenho DXF ou de um ficheiro de traçado HP-GL, os traços desenhados um a um são unidos, e o percurso da ferramenta à volta da placa e dentro dos seus recortes é escrito como HP-GL ou como ficheiro Excellon de fresagem.
* **Duas faces, painéis e furos de alinhamento** (`src/transform.rs`): a camada, os furos e o contorno são virados, rodados em quartos de volta e repetidos num painel em conjunto; os furos de alinhamento ficam fora da placa onde a placa virada os volta a encontrar (um terceiro sobre o eixo distingue-a da mesma placa rodada meia volta).
* **Regras de projeto** (`src/drc.rs`): cobre mais estreito do que a regra, peças de cobre mais próximas do que a regra ou do que a largura da ferramenta, e furos com menos cobre à volta do que a regra, cada um com a sua posição em milímetros.
* **Um ficheiro de definições** (`src/config.rs`): cada valor que se pode afinar é uma definição com um valor por omissão, lida de um ficheiro `chave = valor` (`examples/isocam.cfg`) ou dada com o mesmo nome como opção da linha de comandos.
* **Isolamento forçado** (`src/forced.rs`): com `forced_isolation = yes` é traçado também um percurso a meio de cada espaço em que a ferramenta mais fina não cabe, desde que o espaço não seja mais estreito do que `forced_min_gap_mm`; cada sítio é assinalado com as suas coordenadas e o seu espaço, e os espaços mais estreitos são listados e deixados como estão. Vem desativado: nesse caso esses sítios são apenas assinalados.
* **Percursos mais leves, escala e correção** (`src/optimize.rs`, `src/fiducial.rs`): os picos são retirados e os pontos numa reta (dentro de `optimize_tolerance_mm`) passam a ser um só movimento, ativado por predefinição (0,01 mm e 0,02 mm; com 0 desativa-se); o trabalho pode ser escalado em relação ao centro da placa, e pares de pontos de referência relativos a esse centro (onde uma marca devia estar e onde foi medida) dão uma semelhança (dois pares) ou um ajuste afim (três ou mais, mínimos quadrados) aplicado a cada percurso e a cada furo; é recusado quando a escala se afasta muito de 1 ou uma marca não encaixa.
* **Máscara de soldadura** (`src/mask.rs`): as aberturas são escritas como uma camada Gerber, a partir da camada de máscara do desenho ou dos pads do cobre, cada uma alargada de `mask_margin_mm`; as pistas ficam cobertas.
* **Unidades e pré-visualização** (`src/units.rs`, `src/preview.rs`): um comprimento pode levar `mm`, `mil` ou `in`, o G-code pode ser escrito em polegadas, e `preview` desenha as passagens sobre o cobre como desenho SVG ou como texto.
* **Camadas Gerber antigas** (`src/legacy.rs`): uma camada sem instrução de formato (RS-274D, como muitos programas antigos a escrevem) é lida quando se indica a sua lista de aberturas (`--apertures`, um código D e o seu tamanho por linha) juntamente com o seu formato (`--gerber-format 2.3`, `--gerber-zeros`, `--gerber-units`); é reescrita como Gerber estendido e passa pelo leitor normal. Testado com camadas escritas à mão, não com ficheiros de outros programas.

## 🧩 Papel no EH3D

EH3D-ISOCAM é um programa da EH3D Fabrication Suite. Hoje funciona como biblioteca e linha de comandos; a sua janela, com o aspeto que os outros programas da suíte partilham, está prevista. Lê e escreve ficheiros simples, por isso pode ser usado sozinho ou entre os outros programas.

## 🏛️ Arquitetura

<p align="center">
  <img src="images/architecture.svg" alt="Arquitetura de EH3D-ISOCAM: quem o usa, o que contém e de que depende" width="100%">
</p>


Como as partes deste repositório se encaixam: quem o usa, o que contém e de que depende. A imagem é desenhada a partir de `docs/architecture.json` e verificada com o código.

## ✨ Funcionalidades

* **Biblioteca e linha de comandos:** cada comando de `eh3d-isocam` é uma função da biblioteca, por isso outro programa da suíte pode chamá-la sem lançar um processo.
* **Testes com entradas danificadas:** os testes também dão ficheiros vazios, lixo, números enormes e ficheiros cortados, e nenhum pode entrar em pânico.
* **Sem adivinhar:** um valor sem sentido é um erro que diz qual e porquê, nunca um resultado que apenas parece certo.
* **Sem código unsafe:** `#![forbid(unsafe_code)]` na biblioteca e no programa.
* **Cada valor afinado é uma definição:** nada do que uma pessoa possa querer mudar está escondido no código; um valor mau é um erro com a sua linha.
* **Unidades à escolha:** os comprimentos de um ficheiro de definições ou da linha de comandos podem ser escritos em milímetros, mils ou polegadas; o G-code pode ser escrito em polegadas e o ecrã pode mostrar mils.

## 🔒 Segurança e proteção

| Ameaça | Defesa |
|---|---|
| Um ficheiro danificado, cortado ou vazio | sem pânico: os testes cortam uma entrada válida em cada posição e dão lixo |
| Um número sem sentido (negativo, não finito, absurdamente grande) | recusado com um erro que nomeia o valor; nada é arredondado para algo plausível |
| Uso inseguro de memória | Rust seguro e simples com `forbid(unsafe_code)` |

**O que não é defendido:** um resultado produzido sem erros não é um resultado verificado. Nada aqui conhece os limites de uma máquina e o programa não teve revisão independente. Comunique uma vulnerabilidade em privado (ver [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

A biblioteca é `eh3d_isocam` (`use eh3d_isocam::...`); a linha de comandos é `eh3d-isocam`. Cada comando escreve o resultado na saída padrão e comunica um erro na saída de erros com um código de saída diferente de zero. Não há acesso à rede. O texto da ajuda (`eh3d-isocam help`) está em inglês:

```text
eh3d-isocam isolate LAYER.gtl OUT.nc|OUT.plt     paths round the copper (G-code, or HP-GL for a .plt)
eh3d-isocam check LAYER.gtl                      places where the tool does not fit between pieces of copper
eh3d-isocam info LAYER.gtl                       passes, paths each way round, travel through the air
eh3d-isocam preview LAYER.gtl [OUT.svg|OUT.txt]    the passes drawn over the copper, as SVG or text
eh3d-isocam drc LAYER.gtl [--drills F.ncd]       design rules, with the place of every finding
eh3d-isocam drills IN.ncd OUT.ncd [--align]      read, turn over, panel and add alignment holes
eh3d-isocam route OUTLINE OUT.plt|OUT.ncd        path of the tool round the board
eh3d-isocam mask COPPER.gtl OUT.gts [--mask-layer F] openings of the solder mask, from the mask layer or the pads
eh3d-isocam config                               every setting with its default, as a file
options: --config FILE  --outline FILE  --drills FILE  --force  and every setting by its name (--fine-tool-mm 0.15, --fine-tool-mm 8mil, --forced-isolation yes, --fiducial "0,0,0.1,0.05")
old-style Gerber (no format statement): --apertures FILE  --gerber-format 2.3  --gerber-zeros leading|trailing  --gerber-units inch|mm  --aperture-units mil|mm|inch
```

```text
EH3D-ISOCAM/
├── src/
│   ├── bitmap.rs                                o cobre numa grelha
│   ├── config.rs                                todos os valores ajustáveis, a partir de um ficheiro de definições
│   ├── contour.rs                               os percursos de uma passagem, traçados e aligeirados
│   ├── drc.rs                                   regras de projeto: larguras, distâncias e anéis, com as posições
│   ├── drill.rs                                 furos e contorno da placa, lidos e escritos
│   ├── edt.rs                                   a distância exata ao cobre
│   ├── fiducial.rs                              escala e correção com pontos de referência
│   ├── forced.rs                                percursos nos espaços onde a ferramenta não cabe
│   ├── gcode.rs                                 os percursos como G-code
│   ├── hpgl.rs                                  os percursos como ficheiro de traçado HP-GL
│   ├── isolate.rs                               ferramentas, passagens, cobre restante e folgas estreitas
│   ├── legacy.rs                                camadas Gerber antigas: formato e lista de aberturas
│   ├── lib.rs                                   a biblioteca: o que cada comando usa
│   ├── main.rs                                  a linha de comandos
│   ├── mask.rs                                  as aberturas da máscara de soldadura
│   ├── optimize.rs                              percursos mais leves: picos e pontos em linha
│   ├── order.rs                                 sentido, pontos de início e ordem dos cortes
│   ├── preview.rs                               as passagens desenhadas em SVG ou texto
│   ├── transform.rs                             dupla face, painéis e furos de alinhamento
│   └── units.rs                                 mm, mil e polegada
├── tests/
│   ├── advanced.rs                              isolamento forçado, correção, máscara, unidades, pré-visualização
│   ├── files.rs                                 testes dos ficheiros de furação e contorno
│   ├── isocam.rs                                testes da biblioteca e da linha de comandos
│   ├── layout.rs                                testes de dupla face, painéis, regras e linha de comandos
│   ├── legacy.rs                                testes das camadas Gerber antigas e da linha de comandos
│   └── multi.rs                                 testes de ferramentas, cobre restante, sentido, ordem e definições
├── examples/
│   ├── isocam.cfg                               todas as definições com o valor por omissão, para copiar e alterar
│   └── sample.gtl                               um exemplo para experimentar o programa
├── docs/
│   ├── architecture.json                        dados da imagem de Arquitetura
│   └── structure.json                           dados da imagem de Estrutura
├── tools/                                       verificação da CI e do README e ferramentas de imagens
├── images/                                      banner e as imagens de Arquitetura e Estrutura do repositório
├── .github/                                     fluxos de CI e de publicação, modelos de issues e pull requests
├── eh3d.project.json                            manifesto: versão, papel, maturidade e comando de compilação
├── Cargo.toml Cargo.lock                        o workspace e as suas dependências fixadas
├── CHANGELOG.md                                 o que mudou em cada versão
├── SECURITY.md SUPPORT.md                       como comunicar uma vulnerabilidade e onde perguntar
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   como contribuir, o código de conduta e a licença
```

## ⚙️ Configuração

Cada valor que se pode afinar é uma definição com um valor por omissão: os diâmetros, as passagens e o avanço lateral da ferramenta fina e da grossa, a retirada do cobre restante, a profundidade, o avanço e a velocidade, o sentido, o ponto de partida, o início de cada percurso e a ordem, a ferramenta que corta a placa, os furos de alinhamento, o espelho, a rotação e o painel, e as regras de projeto. `eh3d-isocam config` imprime-as todas como um ficheiro (`examples/isocam.cfg` é esse ficheiro); `--config FICHEIRO` lê um, e qualquer definição pode ser dada na linha de comandos como opção com o mesmo nome (`--fine-tool-mm 0.15`), que prevalece sobre o ficheiro. Um valor sem sentido é um erro que indica a linha. Os limites fixos vivem no código como constantes com nome e estão cobertos por testes. O isolamento forçado, os percursos mais leves, a escala e os pontos de referência, a margem da máscara de soldadura, as unidades e a largura da pré-visualização em texto também são definições.

## 🚀 Início rápido

Precisa de uma cadeia de ferramentas Rust estável (1.88 ou mais recente).

```bash
cargo build --release
cargo test
cargo run --release -- isolate examples/sample.gtl examples/sample.nc --passes 2
```

## 🧪 Testes

O conjunto tem 107 testes (`cargo test`, executados em Windows e Linux no CI). Verificam os casos normais, os casos-limite e as entradas danificadas: ficheiros vazios, lixo e ficheiros cortados nunca provocam pânico. O CI executa ainda `cargo fmt --check`, `cargo clippy` com avisos como erros e `python -I tools/ci_validate.py`.

## 🏗️ Compilação e publicação

A versão está em `Cargo.toml` e em `eh3d.project.json`; cada versão tem a sua entrada em [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` compilam o programa, `test.bat` / `test.sh` executam os testes e `run.bat` / `run.sh` executam-no com um exemplo. As versões são publicadas no GitHub.

## 📂 Estrutura do repositório

<p align="center">
  <img src="images/repository-structure.svg" alt="Estrutura do repositório EH3D-ISOCAM" width="100%">
</p>

<details>
<summary>Versão em texto da árvore</summary>

```text
EH3D-ISOCAM/
├── src/
│   ├── bitmap.rs                                o cobre numa grelha
│   ├── config.rs                                todos os valores ajustáveis, a partir de um ficheiro de definições
│   ├── contour.rs                               os percursos de uma passagem, traçados e aligeirados
│   ├── drc.rs                                   regras de projeto: larguras, distâncias e anéis, com as posições
│   ├── drill.rs                                 furos e contorno da placa, lidos e escritos
│   ├── edt.rs                                   a distância exata ao cobre
│   ├── fiducial.rs                              escala e correção com pontos de referência
│   ├── forced.rs                                percursos nos espaços onde a ferramenta não cabe
│   ├── gcode.rs                                 os percursos como G-code
│   ├── hpgl.rs                                  os percursos como ficheiro de traçado HP-GL
│   ├── isolate.rs                               ferramentas, passagens, cobre restante e folgas estreitas
│   ├── legacy.rs                                camadas Gerber antigas: formato e lista de aberturas
│   ├── lib.rs                                   a biblioteca: o que cada comando usa
│   ├── main.rs                                  a linha de comandos
│   ├── mask.rs                                  as aberturas da máscara de soldadura
│   ├── optimize.rs                              percursos mais leves: picos e pontos em linha
│   ├── order.rs                                 sentido, pontos de início e ordem dos cortes
│   ├── preview.rs                               as passagens desenhadas em SVG ou texto
│   ├── transform.rs                             dupla face, painéis e furos de alinhamento
│   └── units.rs                                 mm, mil e polegada
├── tests/
│   ├── advanced.rs                              isolamento forçado, correção, máscara, unidades, pré-visualização
│   ├── files.rs                                 testes dos ficheiros de furação e contorno
│   ├── isocam.rs                                testes da biblioteca e da linha de comandos
│   ├── layout.rs                                testes de dupla face, painéis, regras e linha de comandos
│   ├── legacy.rs                                testes das camadas Gerber antigas e da linha de comandos
│   └── multi.rs                                 testes de ferramentas, cobre restante, sentido, ordem e definições
├── examples/
│   ├── isocam.cfg                               todas as definições com o valor por omissão, para copiar e alterar
│   └── sample.gtl                               um exemplo para experimentar o programa
├── docs/
│   ├── architecture.json                        dados da imagem de Arquitetura
│   └── structure.json                           dados da imagem de Estrutura
├── tools/                                       verificação da CI e do README e ferramentas de imagens
├── images/                                      banner e as imagens de Arquitetura e Estrutura do repositório
├── .github/                                     fluxos de CI e de publicação, modelos de issues e pull requests
├── eh3d.project.json                            manifesto: versão, papel, maturidade e comando de compilação
├── Cargo.toml Cargo.lock                        o workspace e as suas dependências fixadas
├── CHANGELOG.md                                 o que mudou em cada versão
├── SECURITY.md SUPPORT.md                       como comunicar uma vulnerabilidade e onde perguntar
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   como contribuir, o código de conduta e a licença
```

</details>

## 🗺️ Roteiro e limites conhecidos

**Limites conhecidos hoje**

* Ainda não tem janela: existem apenas a biblioteca e a linha de comandos.
* Não foi experimentado numa máquina nem com uma coleção de ficheiros reais, e não teve revisão independente.
* Os percursos são exatos a meia célula, não à própria curva; não há estratégia Voronoi, os arcos não são ajustados, por decisão (uma curva são muitos movimentos curtos), um percurso forçado deixa uma ferramenta mais larga do que o espaço tocar no cobre dos dois lados, a correção com pontos de referência move os percursos mas mantém o tamanho do desvio da ferramenta, a escala é em relação ao centro da placa, não há camada de serigrafia e nenhuma placa foi fresada com estes percursos.
* A máscara de soldadura segue os pads tal como estão desenhados (círculos, retângulos e ovais); as pistas ficam cobertas e uma margem negativa não encolhe uma região.
* A retirada do cobre restante faz-se com anéis de uma ferramenta mais larga a uma distância do cobre: os sítios mais estreitos do que essa ferramenta mantêm o seu cobre, e não segue a borda da placa.
* Os troços omitidos para uma ferramenta mais fina são decididos numa grelha com duas células de margem; prefere-se cortar um troço duas vezes a deixá-lo por cortar.
* Os ficheiros de furação são escritos em milésimos de polegada inteiros (convenção dos programas Bungard): um diâmetro `T1C0.8` sem unidade é lido como milímetros a partir de 0,2 e como polegadas abaixo disso; os ficheiros com ranhuras ou percursos de fresagem deixam-nos de fora com uma nota. Os ficheiros escritos só foram relidos pelos leitores desta suite, não por outros programas nem máquinas.
* As regras de projeto não arredondam os cantos de uma pastilha quadrada e não verificam o cobre em relação à borda da placa nem à máscara de soldadura; a verificação dos anéis toma a pastilha como desenhada na camada Gerber.

**A seguir**

* Acrescentar a janela no aspeto comum aos programas da suite, nos oito idiomas, sobre a pré-visualização que hoje existe como texto e SVG.
* A estratégia Voronoi; comparar a correção com pontos de referência com medições de uma câmara.
* Camadas de serigrafia.
* Comparar os percursos com placas realmente fresadas e experimentá-lo com ficheiros reais e com os outros programas da suite.

## 🌍 Idiomas

O README e os textos existem em inglês, espanhol, francês, italiano, alemão, chinês, japonês e português europeu.

## 🔗 Projetos relacionados

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): os leitores de Gerber, Excellon e G-code
* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): os formatos de ficheiros das máquinas Bungard: furação, traçado e contornos
* [EH3D-GERBERVIEWER](https://github.com/JuanenRac/EH3D-GERBERVIEWER): o visualizador de camadas Gerber
* [EH3D-TOOL-MANAGER](https://github.com/JuanenRac/EH3D-TOOL-MANAGER): o armazém de ferramentas partilhado pelos programas CAM
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): a verificação do G-code antes de chegar a uma máquina
* [EH3D-AUTOLEVEL](https://github.com/JuanenRac/EH3D-AUTOLEVEL): o mapa de alturas e a correção do G-code
* [EH3D-VISION-STUDIO](https://github.com/JuanenRac/EH3D-VISION-STUDIO): as câmaras, os fiduciais e a inspeção

## 📚 Documentação

* [Registo de alterações](CHANGELOG.md)
* [Dados de arquitetura e estrutura](docs/architecture.json): os dados a partir dos quais as duas imagens são desenhadas.

## 🤝 Contribuir e comunidade

* Antes de um pull request execute `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` e `python -I tools/ci_validate.py`: tudo tem de passar. Veja [CONTRIBUTING.md](CONTRIBUTING.md) e o [código de conduta](CODE_OF_CONDUCT.md).
* O README existe em oito idiomas: uma alteração ao texto faz-se nos oito ficheiros, e as duas imagens são desenhadas a partir de `docs/architecture.json` e `docs/structure.json` por `tools/make_architecture.py` e `tools/make_structure.py`.
* Comunique uma vulnerabilidade em privado: [SECURITY.md](SECURITY.md). Perguntas, ideias e relatos: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENÇA

Software: GPL-3.0-or-later. Ver [LICENSE](LICENSE). Os projetos de hardware da suite usam a CERN OHL-S.
