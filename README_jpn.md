<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-ISOCAM banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-ISOCAM
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | 🇯🇵 <b>日本語</b> | <a href="README_por.md">🇵🇹 Português</a></p>

### Gerber レイヤーからの銅箔の絶縁ミリング：正確なオフセット、複数パス、隙間の確認

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.7-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**正直さの確認 - 現在実際に動くもの：** Rustライブラリとコマンドラインプログラム（`eh3d-isocam`）で、コンピュータ上で動くテストが294件あります。Gerberレイヤーの銅の周囲に1本または複数のミリング工具の経路を描き、残りの銅を除去し、切削の向き・開始点・順序を選び、ドリルファイルと基板外形を読み書きし、基板を両面およびパネル用に配置し、設計ルールを確認し、GコードまたはHP-GLを書き出します。ウィンドウ（`--features gui` でビルド）ではレイヤー、ドリルファイル、基板外形、マスクレイヤー、基準点の表を開き、基板を裏返してパネルに並べ、銅・穴・外形・経路を描画し、コマンドラインと同じファイルを書き出せますが、画面ではまだ確認しておらず、実機では試しておらず、実際のファイル群での確認もしていません。

---

> **安全とセキュリティの状況：** このプログラムは機械を制御せず、安全システムでもありません。出力は、機械で何かを実行する前に人が確認する必要があります。ここにあるものは実機で動かしたことがありません。


## 📋 ひと目で

| | |
|---|---|
| バージョン | `0.0.7` |
| 成熟度 | 足場 |
| テスト | 294 |
| 技術スタック | Rust |
| 実機での動作 | 未試験：ここにあるものは機械で動かしたことがありません |

**移動先：** [概要](#-概要) · [アーキテクチャ](#-アーキテクチャ) · [クイックスタート](#-クイックスタート) · [テスト](#-テスト) · [構成](#-リポジトリの構成) · [関連](#-関連プロジェクト)

## 🚦 プロジェクトの状況

各行はこのリポジトリの一部分で、信号は今日どこまで進んでいるかを示します。計画ではなく、リポジトリが示せるもの（テスト、ビルド、実行）に基づいて書かれています。

🟢 完了・検証済み · 🟡 動作する（既知の制限あり） · 🟠 一部のみ、または未検証 · 🔴 未実装または動作しない

| 領域 | 状態 | ここでの意味 |
|---|---|---|
| コア | 🟡 | Gerberレイヤーの銅の周囲に1本または複数のミリング工具の経路を描き、残りの銅を除去し、切削の向き・開始点・順序を選び、工具が入らない狭い隙間を報告し、GコードまたはHP-GLを書き出します。通常の入力と壊れた入力でコンピュータ上でテスト済みで、実際のファイル群ではテストしていません。 |
| ファイルと配置 | 🟡 | ドリルファイルと基板外形を読み書きし、基板を両面およびパネル用に配置し、位置合わせ穴を追加し、設計ルールを検出位置つきで確認します。コンピュータ上でテスト済みで、他のプログラムが書いたファイルや実機では試していません。 |
| 強制アイソレーション・補正・マスク | 🟡 | 工具が入らない隙間に沿う強制経路、軽い経路、基準点によるスケールと補正、ソルダーマスクの開口部、単位とパスのプレビュー。生成したレイヤーと破損入力でコンピュータ上でテストしましたが、実際の基板や機械では試しておらず、補正はカメラの実測値と比較していません。 |
| コマンドライン | 🟡 | `eh3d-isocam` はライブラリと同じ関数を実行して結果を表示します。エラーはメッセージと 0 以外の終了コードで返ります。 |
| テスト | 🟡 | 294 件のテストが通ります（`cargo test`）。通常ケース、境界ケース、破損入力を含みます。 |
| セキュリティ | 🟡 | 安全な Rust のみ（`forbid(unsafe_code)`）。受け取ったファイルでパニックせず、不正な値は理由付きのエラーになります。独立したレビューはありません。 |
| ウィンドウ | 🟠 | 作成済み（egui、`--features gui`）：レイヤーを開くかドロップ、拡大縮小と移動、主なパラメーターの編集、書き出し。ロジックはテスト済みですが、まだビルドも画面での確認もしていません。 |
| 実機 | 🔴 | 未試験：ここにあるものは機械で実行したことがありません。 |
| ドキュメント | 🟡 | 8 言語の README、変更履歴、`docs/` から描いた 2 枚の図。 |

## 🎯 概要

* **格子上の銅箔**（`bitmap`）：パッド（円、長方形、長円）、配線、領域を、選んだ大きさのセルに描きます。格子には重なりがないため、重なった配線とパッドは 1 つの塊になり、特別な場合分けは不要です。メモリに収まらない格子は拒否されます。
* **正確な距離**（`eh3d-raster`）：各セルから最も近い銅箔までの距離を正確に計算し（総当たりと照合してテスト）、1 パスの経路は距離が工具半径に等しい線として閉じたポリラインで追跡し、半セルの許容差で間引きます。以降のパスはそれぞれ 1 ステップオーバーずつ外側です。
* **工具にできないこと**（`isolate`）：離れた 2 つの銅箔が工具幅より近いと、その間を通る経路はありません。`check` がその場所を一覧し、確認するまで（または `--force`）`isolate` は G コードを書きません。G コード（`src/gcode.rs`）は経路間で工具を持ち上げ、選んだ深さまで下げ、次に最も近い経路を切ります。
* **Bungard 機械向けの HP-GL**（`src/hpgl.rs`）：出力ファイルの拡張子が `.plt` のとき、各パスを 1 mm あたり 40 単位の HP-GL プロット ファイルとして書き出し、パスごとに 1 本のペンを使います。Bungard 機械のルーティング プログラムが読む形式です。
* **複数の工具と残り銅**（`isolate`）：プランには最大4本の工具があり、最も太いものが先に切削します。太い工具が細い工具の経路に沿ってすでに切削した箇所は省かれ（残りは開いた区間になります）、太い工具の輪（それぞれ前の輪と重なる）でパスの外側に残った銅を取り除けます。工具の幅が変わるとGコードは工具交換のために停止します。 `around_pads_min_mm` を 0 より大きくすると、幅の広い工具はその幅以上のパッドの周りのリングだけを切削し、最も細い工具が残り(配線と小さなパッド)を担当します。
* **向き・開始点・順序**（`order`）：経路は銅が工具の右側になるように描かれます（時計回りの主軸でのダウンカット）。設定で、これを逆にする（アップカット）、時計回りまたは反時計回りに固定する、閉じた経路を最も近い点・最も低い点・描かれた位置から始める、工具の出発点を決める、経路を描いた順・近い順・順序の一部を反転してさらに短くした順に並べる、ことができます。`info`は各向きの経路数と空中での移動距離を表示します。
* **穴と基板外形**（`src/drill.rs`）：ドリルファイルを読み（Bungardのミル単位のファイルはBungard形式の読み取り器で、メートル法のファイルはスイートの読み取り器で）、整数ミルで書き出します。外形はGerber外形レイヤー、DXF図面、HP-GLプロットファイルから読み、1本ずつ描かれた線は結合され、基板の周囲と切り抜き内側の工具経路はHP-GLまたはルーティング用Excellonファイルとして書き出されます。255 を超える番号の工具を持つドリルファイルでは、通知とともに空き番号へ振り直されるため、2 つの工具が 1 つになることはありません。
* **両面・パネル・位置合わせ穴**（`transform`）：レイヤー・穴・外形をまとめて裏返し、90度単位で回転し、パネル上に繰り返します。位置合わせ穴は、裏返した基板が同じ位置に見つけられるよう基板の外側に置かれます（軸上の3つ目の穴で、半回転した同じ基板と区別できます）。
* **設計ルール**（`drc`）：ルールより細い銅、ルールまたは工具の幅より近い銅片、ルールより銅の少ない穴の周囲を、ミリメートルの位置つきで報告します。
* **1つの設定ファイル**（`src/config.rs`）：調整できる値はすべて既定値のある設定で、`キー = 値`形式のファイル（`examples/isocam.cfg`）または同名のコマンドラインオプションから読み込まれます。
* **強制アイソレーション**（`forced`）：`forced_isolation = yes` にすると、最も細い工具が入らない隙間の中央にも、その隙間が `forced_min_gap_mm` より狭くない限り経路を引きます。各箇所は座標と隙間の幅つきで報告され、それより狭い隙間は一覧に出して手を付けません。初期値はオフで、その場合これらの箇所は報告されるだけです。
* **問題箇所・許容差・位置合わせ穴の距離**（`problems`、`src/config.rs`）：`check` は、何も止めない警告として、`problem_corner_angle_deg` より鋭い内角（丸い工具は切り欠きの先端に銅を残します。既定は 60 度、0 で無効）と、`problem_track_gap_factor` 掛ける工具径より近くに 2 つのパッドがあるトラック（既定は 1.5、0 で無効）も一覧にします。どちらの基準も設定であり、コードに固定されていません。`conflict_tolerance_mm`（既定は 0：どの隙間も受け入れません）は、工具より最大でその分だけ狭い隙間を `check` と `isolate` で受け入れます。位置合わせ穴と基板の距離は `alignment_margin_mm`（既定は 5 mm。この種の他のプログラムは 10 mm に置きます。それは `alignment_margin_mm = 10` です）。
* **ブリッジ、ネット、穴パターン、領域の除去** （`src/drill.rs`、`isolate`、`transform`、`problems`、`legacy`）：`route` は基板をシートに保持するブリッジを残せます（閉じた経路ごとに `tab_count`、初期値は0。残す材料の幅は `tab_width_mm`、ブリッジの間隔は `tab_min_length_mm`）。切削は各ブリッジの中心の手前、ブリッジ幅の半分と工具径の半分の位置で止まります。`isolate --net X,Y` は、点を含むネットだけを工具 `net_tool_mm` で絶縁します。`rubout-area OUT --area X0,Y0,X1,Y1` は、工具 `rubout_lines_tool_mm` の平行線（向きは `rubout_lines_direction`）を書き出して領域を除去します。位置合わせ穴は2、3、6個で、基板の外側、または `alignment_pattern = special` で中心まわりのパターン（ペアの間隔は `alignment_special_a_mm`、追加の4個は `alignment_special_b_mm`）に置けます。競合許容差を設定すると、`check` と `isolate` は、許容差の2倍に `tolerance_warning_margin_mm` を足した値より細い配線と長方形パッドを警告します。旧形式のGerberレイヤーの形式推定は、ファイルが示す単位と基板の大きさを使うようになり、アパーチャ一覧は1つのセルに両辺を書けます（`60x40`）。
* **パッド専用の許容差、輪郭による塗りつぶし、ファイル種別、DXF とプロッタのペン** （`tolerance`、`fill`、`src/filetype.rs`、`src/dxf.rs`、`src/hpgl.rs`）：`conflict_tolerance_scope = pads`（既定は従来どおり `all`）にすると、競合許容差は隣にパッドがある隙間だけに適用され、細い銅の警告もパッドだけを見ます。`rubout-area OUT [LAYER.gtl] --contour FILE`（または `--area`）は、ファイルの閉じたパスを偶奇規則で塗りつぶし（内側のパスは穴）、`rubout_lines_tool_mm` の往復線を引きます。レイヤを渡すと線は銅に合わせて切り取られ、工具全体が銅の上にあり、さらに `rubout_fill_margin_mm` だけ内側になります。実は Excellon・HP-GL・DXF のファイルであるレイヤ（または Gerber レイヤであるドリルファイル）は名前を挙げて拒否され、`kind FILE...` は各ファイルの種類を答えます。`isolate` と `rubout-area` は `.dxf` なら DXF 図面を書きます。`drills IN OUT.plt` はドリルごとに 1 本のペンで穴をプロットし（`hpgl_pen_map = 1:2, 3:1`。指定のないドリルは空いている最小のペンを使います）、`hpgl_pen_speed_cm_s`（0 = 機械側の速度、既定）はあらゆるプロットにペン速度を書き込みます。 `hpgl_isocam_style`(オン)は、元のプログラムと同じ形式でプロットを書き出します。点ごとに `PA` を使い、約75桁ごとに CR LF で行を区切ります。オフにするとコンパクトな形式で書き出します。
* **両面基板とシンプルなドリルファイル** (`src/hpgl.rs`, `src/drill.rs`, `src/filetype.rs`): `hpgl_shift_negative = no` はプロットに基板の座標（負の値を含む）をそのまま残し、プロットとドリルファイルが同じ原点を共有します。`isolate` に `plot_outline = yes` と `--outline ファイル` を与えると、外形のまわりのカット（`route_tool_mm` の工具）がプロットの先頭に `hpgl_outline_pen` のペンで入り、アイソレーションは `hpgl_isolation_pen` のペンになります。`--mirror vertical` は外形の中央を軸に穴を反転します。`ncd_isocam_style = yes` は Bungard のプログラムと同じ形でドリルファイルを書き（`M48` と `%`、工具は番号のみ、繰り返しの座標は省略、CR LF、`M00`）、`ncd_digits` は座標の前をゼロで埋めます。`M48` 行のないドリルファイルも Excellon として認識されます。 プロットを移動する場合、`hpgl_margin_mm` が1枚の基板の絶縁プロットと穴プロットに共通の移動量を決め、位置がずれないようにします。`spindle_wait_s` は G コードでスピンドル始動のたびに待機（`G4`）を書き込みます。
* **軽い経路、スケールと補正**（`optimize`、`fiducial`）：スパイクを取り除き、直線上の点（`optimize_tolerance_mm` 以内）を1つの動きにまとめます（どちらも既定で有効：0.01 mm と 0.02 mm、0 で無効）。作業全体を基板の中心を基準にスケール変更でき、基準点のペア（基板の中心からの相対座標で、マークがあるべき位置と実測位置）から相似変換（2組）またはアフィン近似（3組以上、最小二乗）を求めて、すべての経路と穴に適用します。スケールが1から大きく外れる場合やマークが合わない場合は拒否されます。
* **ソルダーマスク**（`mask`）：開口部を Gerber レイヤーとして書き出します。設計のマスクレイヤーまたは銅のパッドから作り、それぞれ `mask_margin_mm` だけ広げます。配線は覆われたままです。
* **単位とプレビュー**（`eh3d-raster`、`src/preview.rs`）：長さには `mm`、`mil`、`in` を付けられ、G-code はインチでも書き出せます。`preview` は銅の上にパスを SVG 図またはテキストで描きます。
* **旧形式のガーバーレイヤー**（`legacy`）：書式指定のないレイヤー（古いプログラムの多くが書く RS-274D）は、アパーチャ一覧（`--apertures`、1行に D コードとそのサイズ）と書式（`--gerber-format 2.3`、`--gerber-zeros`、`--gerber-units`）を与えると読み込めます。拡張ガーバーに書き直されて通常の読み取り器に渡されます。手書きのレイヤーでテストしており、他のプログラムのファイルでは試していません。
* **読み込みとコード別の通知（コマンドライン）：** ファイルはサイズ上限（`--max-bytes N` or `max_input_mb`、既定 64 MiB）付きで読み込まれ、UTF-8 でないテキストは警告（`input-not-utf8`）付きで Latin-1 として読みます。Gerber リーダーの通知は安定したコード付きで表示され、`--hide CODE[,CODE]` でそのコードを省けます。軽量化したパスの注記と切断のブリッジの注記は、何かが変わったときだけ表示されます。 `arc_tolerance_mm` はレイヤーの円弧をどれだけ忠実にたどるかを決めます。

## 🧩 EH3D での役割

EH3D-ISOCAM は EH3D Fabrication Suite のプログラムの一つです。ライブラリとコマンドラインとして動作し、`--features gui` でビルドすると、スイートの他のプログラムと共通の外観のウィンドウとしても動作します。単純なファイルを読み書きするため、単独でも他のプログラムの間でも使えます。

## 🏛️ アーキテクチャ

<p align="center">
  <img src="images/architecture.svg" alt="EH3D-ISOCAM のアーキテクチャ：誰が使い、何を持ち、何に依存するか" width="100%">
</p>


このリポジトリの各部分の関係：誰が使い、何を持ち、何に依存するか。図は `docs/architecture.json` から描かれ、コードと照合されます。

## ✨ 機能

* **ライブラリとコマンドライン：** `eh3d-isocam` の各コマンドはライブラリの関数なので、スイートの他のプログラムがプロセスを起動せずに呼び出せます。
* **破損入力のテスト：** 空のファイル、ゴミデータ、巨大な数値、途中で切れたファイルも与え、どれもパニックしてはいけません。
* **推測しない：** 意味のない値は、どの値かと理由を示すエラーになり、見かけだけ正しい結果にはなりません。
* **unsafe コードなし：** ライブラリとプログラムに `#![forbid(unsafe_code)]`。
* **調整した値はすべて設定です：** 人が変えたくなる値がコードに埋め込まれることはなく、不正な値は行番号つきのエラーになります。
* **単位を選べる：** 設定ファイルやコマンドラインの長さは、ミリメートル、ミル、インチで書けます。G-code はインチで書き出せ、画面表示はミルにもできます。
* **より安全なファイル：** G コードは標準のヘッダー（ジェネレーター、単位、元ファイルとその SHA-256）で始まり、実行の入力でもある出力は拒否され、32767 単位を超える座標のプロットは拒否され、`--units mm|inch` は単位のないドリルファイルの単位を与え（極小の穴で単位のないファイルはこれがないと拒否されます）、別々の線からなる外形は索引で結合するので 5 万本の線分も一瞬です。
* **出力の確認と説明:** ドリルファイルはコメントで、座標が小数点のない整数ミル(0.001インチ)であること、丸めの誤差が最大どれだけかを示します。テストは、パスのG-codeとコマンドラインのG-codeをスイートのリーダーで読み直し、警告、材料への早送り、スピンドルが止まったままの切削が1つも見つからないことを確かめます。
* **表からの基準マーク：** `--fiducials-csv FILE` はマークの組（設計 x、設計 y、測定 x、測定 y。見出し行と `#` で始まる行は除く）を読み込むので、`eh3d-vision align --isocam-args` の出力をそのまま使えます。

## 🔒 セキュリティと安全

| 脅威 | 対策 |
|---|---|
| 破損・途中切れ・空のファイル | パニックなし：テストは有効な入力をあらゆる位置で切り、ゴミデータも与えます |
| 意味のない数値（負、無限、極端に大きい） | 値を示すエラーで拒否され、もっともらしい値に丸められることはありません |
| 安全でないメモリ使用 | `forbid(unsafe_code)` の安全な Rust のみ |

**防御していないこと：** エラーなく作られた結果は、検証された結果ではありません。ここには機械の限界を知るものはなく、プログラムは独立したレビューを受けていません。脆弱性は非公開で報告してください（[SECURITY.md](SECURITY.md) 参照）。

## 🔌 インターフェース

ライブラリは`eh3d_isocam`（`use eh3d_isocam::...`）、コマンドラインは`eh3d-isocam`です。各コマンドは結果を標準出力に表示し、エラーは標準エラー出力に非ゼロの終了コードで報告します。ネットワークアクセスはありません。ヘルプ（`eh3d-isocam help`）の文章は英語です。

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
│   ├── board.rs                                 コマンドラインとウィンドウが共有する処理
│   ├── browse.rs                                ファイルを開くウィンドウのファイル一覧
│   ├── config.rs                                調整可能な値はすべて設定ファイルから
│   ├── draw.rs                                  ウィンドウが描くもの：図形と領域の三角形
│   ├── drill.rs                                 穴と基板外形の読み書き
│   ├── dxf.rs                                   パスを DXF 図面として出力
│   ├── filetype.rs                              入力ファイルの種類の判別
│   ├── gcode.rs                                 パスをGコードに
│   ├── gui.rs                                   ウィンドウ（egui、gui 機能）：状態を描き、操作を伝える
│   ├── hpgl.rs                                  パスをHP-GLプロットファイルに
│   ├── lang.rs                                  ウィンドウの文言（8言語）
│   ├── lib.rs                                   ライブラリ：各コマンドが使うもの
│   ├── main.rs                                  コマンドライン
│   ├── preview.rs                               パスを SVG またはテキストで描画
│   ├── viewport.rs                              画像の拡大縮小と移動（単純な数値）
│   └── window.rs                                画面なしのウィンドウの状態：レイヤー、パラメーター、パス、書き出し、ステータス
├── tests/
│   ├── advanced.rs                              強制アイソレーション・補正・マスク・単位・プレビュー
│   ├── cli_reading.rs                           ファイル読み込みのテスト：サイズ上限、UTF-8以外のテキスト、コード別の通知
│   ├── files.rs                                 ドリル・外形ファイルのテスト
│   ├── isocam.rs                                ライブラリとコマンドラインのテスト
│   ├── kinds.rs                                 パッド許容差・輪郭塗り・ファイル種別・ペン・DXF
│   ├── layout.rs                                両面・パネル・ルール・コマンドラインのテスト
│   ├── multi.rs                                 工具・残り銅・向き・順序・設定のテスト
│   ├── problems.rs                              問題箇所・許容差・位置合わせ穴のテスト
│   ├── readme.rs                                READMEとrunスクリプトが約束する内容を実行するテスト
│   └── window_cli.rs                            gui 機能なしのビルドがコマンドラインのままであることのテスト
├── examples/
│   ├── isocam.cfg                               すべての設定と既定値（コピーして変更用）
│   └── sample.gtl                               試すためのサンプル
├── docs/
│   ├── architecture.json                        アーキテクチャ図のデータ
│   └── structure.json                           構成図のデータ
├── tools/                                       CI と README のチェック、図のツール
├── images/                                      バナーとアーキテクチャ・リポジトリ構成の図
├── .github/                                     CI とリリースのワークフロー、Issue と PR のテンプレート
├── eh3d.project.json                            マニフェスト：バージョン、役割、成熟度、ビルドコマンド
├── Cargo.toml Cargo.lock                        ワークスペースと固定された依存関係
├── rust-toolchain.toml                          固定されたRustツールチェーン
├── CHANGELOG.md                                 各バージョンの変更点
├── SECURITY.md SUPPORT.md                       脆弱性の報告方法と問い合わせ先
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   貢献方法、行動規範、ライセンス
```

## ⚙️ 設定

調整できる値はすべて既定値のある設定です。細い工具と太い工具の直径・パス数・ステップオーバー、残り銅の除去、深さ・送り・回転数、向き、出発点、各経路の開始点と順序、基板を切り抜く工具、位置合わせ穴、ミラー・回転・パネル、設計ルールが含まれます。`eh3d-isocam config`はそれらをすべてファイルとして出力します（`examples/isocam.cfg`がそのファイルです）。`--config ファイル`で読み込み、どの設定も同名のオプション（`--fine-tool-mm 0.15`）でコマンドラインから指定でき、ファイルより優先されます。意味のない値は行番号つきのエラーになります。固定の上限はコード内の名前つき定数で、テストで確認されています。強制アイソレーション、軽い経路、スケールと基準点、ソルダーマスクの余白、単位、テキストプレビューの幅も設定です。

## 🚀 クイックスタート

安定版の Rust ツールチェーン（1.88 以降）が必要です。

```bash
cargo build --release
cargo test
cargo run --release -- isolate examples/sample.gtl examples/sample.nc --passes 2
cargo run --release --features gui   # the window
```

## 🧪 テスト

テストは 294 件あります（`cargo test`、CI では Windows と Linux で実行）。通常ケース、境界ケース、破損入力を確認し、空のファイル、ゴミデータ、途中で切れたファイルでもパニックしません。CI は `cargo fmt --check`、警告をエラーにした `cargo clippy`、`python -I tools/ci_validate.py` も実行します。

## 🏗️ ビルドとリリース

バージョンは `Cargo.toml` と `eh3d.project.json` にあり、各バージョンは [CHANGELOG.md](CHANGELOG.md) に項目があります。`build.bat` / `build.sh` でビルド、`test.bat` / `test.sh` でテスト、`run.bat` / `run.sh` でサンプル実行します。リリースは GitHub で公開されます。

## 📂 リポジトリの構成

<p align="center">
  <img src="images/repository-structure.svg" alt="EH3D-ISOCAM のリポジトリ構成" width="100%">
</p>

<details>
<summary>ツリーのテキスト版</summary>

```text
EH3D-ISOCAM/
├── src/
│   ├── board.rs                                 コマンドラインとウィンドウが共有する処理
│   ├── browse.rs                                ファイルを開くウィンドウのファイル一覧
│   ├── config.rs                                調整可能な値はすべて設定ファイルから
│   ├── draw.rs                                  ウィンドウが描くもの：図形と領域の三角形
│   ├── drill.rs                                 穴と基板外形の読み書き
│   ├── dxf.rs                                   パスを DXF 図面として出力
│   ├── filetype.rs                              入力ファイルの種類の判別
│   ├── gcode.rs                                 パスをGコードに
│   ├── gui.rs                                   ウィンドウ（egui、gui 機能）：状態を描き、操作を伝える
│   ├── hpgl.rs                                  パスをHP-GLプロットファイルに
│   ├── lang.rs                                  ウィンドウの文言（8言語）
│   ├── lib.rs                                   ライブラリ：各コマンドが使うもの
│   ├── main.rs                                  コマンドライン
│   ├── preview.rs                               パスを SVG またはテキストで描画
│   ├── viewport.rs                              画像の拡大縮小と移動（単純な数値）
│   └── window.rs                                画面なしのウィンドウの状態：レイヤー、パラメーター、パス、書き出し、ステータス
├── tests/
│   ├── advanced.rs                              強制アイソレーション・補正・マスク・単位・プレビュー
│   ├── cli_reading.rs                           ファイル読み込みのテスト：サイズ上限、UTF-8以外のテキスト、コード別の通知
│   ├── files.rs                                 ドリル・外形ファイルのテスト
│   ├── isocam.rs                                ライブラリとコマンドラインのテスト
│   ├── kinds.rs                                 パッド許容差・輪郭塗り・ファイル種別・ペン・DXF
│   ├── layout.rs                                両面・パネル・ルール・コマンドラインのテスト
│   ├── multi.rs                                 工具・残り銅・向き・順序・設定のテスト
│   ├── problems.rs                              問題箇所・許容差・位置合わせ穴のテスト
│   ├── readme.rs                                READMEとrunスクリプトが約束する内容を実行するテスト
│   └── window_cli.rs                            gui 機能なしのビルドがコマンドラインのままであることのテスト
├── examples/
│   ├── isocam.cfg                               すべての設定と既定値（コピーして変更用）
│   └── sample.gtl                               試すためのサンプル
├── docs/
│   ├── architecture.json                        アーキテクチャ図のデータ
│   └── structure.json                           構成図のデータ
├── tools/                                       CI と README のチェック、図のツール
├── images/                                      バナーとアーキテクチャ・リポジトリ構成の図
├── .github/                                     CI とリリースのワークフロー、Issue と PR のテンプレート
├── eh3d.project.json                            マニフェスト：バージョン、役割、成熟度、ビルドコマンド
├── Cargo.toml Cargo.lock                        ワークスペースと固定された依存関係
├── rust-toolchain.toml                          固定されたRustツールチェーン
├── CHANGELOG.md                                 各バージョンの変更点
├── SECURITY.md SUPPORT.md                       脆弱性の報告方法と問い合わせ先
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   貢献方法、行動規範、ライセンス
```

</details>

## 🗺️ ロードマップと既知の制限

**現時点で分かっている制限**

* ウィンドウは画面でまだ確認しておらず、表示するパラメーター以外は既定の設定を使い、`check`、`drc`、`info`、`preview`、`rubout-area` の各コマンドには対応していません。これらはコマンドラインで行います。
* 実機でも実際のファイル群でも試しておらず、独立したレビューも受けていません。
* 経路の精度は半セル分で、曲線そのものではありません。ボロノイ方式はなく、円弧のフィッティングは意図的にしません（曲線は多数の短い動きです）。強制経路では、隙間より太い工具が両側の銅に触れます。基準点による補正は経路を動かしますが、工具のオフセット量は変わりません。スケールは基板の中心を基準にします。シルク層はなく、この経路で基板を削った例もありません。
* ソルダーマスクは描かれたとおりのパッド（円、長方形、長円）に従い、配線は覆われたままで、負の余白は領域を縮めません。
* 残り銅の除去は銅から離れた位置での太い工具1本の輪によるもので、その工具より狭い場所には銅が残り、基板の縁には沿いません。
* 細い工具のために省く区間は2セル分の余裕を持つグリッドで判断され、切り残すより二重に切ることを選びます。
* ドリルファイルは整数ミルで書き出されます（Bungardプログラムの慣例）。単位のない直径`T1C0.8`は0.2以上ならミリメートル、それ未満ならインチとして読みます。長穴やルーティング経路を含むファイルでは、それらを注記つきで除外します。書き出したファイルはこのスイートの読み取り器でしか読み戻しておらず、他のプログラムや機械では確認していません。
* 設計ルールは角形パッドの角を丸めず、銅と基板の縁やソルダーマスクとの関係は確認しません。リングの確認はGerberレイヤーに描かれたパッドをそのまま使います。

**次のステップ**

* ウィンドウを画面で確認し、コマンドラインの確認（`check`、`drc`、プレビュー）もウィンドウで行えるようにする。
* ボロノイ方式。基準点による補正をカメラの実測値と比較する。
* シルク層。
* 実際に削った基板と経路を比較し、実際のファイルやスイートの他のプログラムで試す。

## 🌍 言語

README とテキストは、英語、スペイン語、フランス語、イタリア語、ドイツ語、中国語、日本語、ヨーロッパポルトガル語で提供されています。

## 🔗 関連プロジェクト

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): Gerber、Excellon、G コードのリーダー
* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): Bungardマシンのファイル形式：ドリル・プロット・外形ファイル
* [EH3D-GERBERVIEWER](https://github.com/JuanenRac/EH3D-GERBERVIEWER): Gerber レイヤー ビューアー
* [EH3D-TOOL-MANAGER](https://github.com/JuanenRac/EH3D-TOOL-MANAGER): CAM プログラム共通の工具庫
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): 機械に届く前の G コード検査
* [EH3D-AUTOLEVEL](https://github.com/JuanenRac/EH3D-AUTOLEVEL): 高さマップと G コードの補正
* [EH3D-VISION-STUDIO](https://github.com/JuanenRac/EH3D-VISION-STUDIO): カメラ、基準マーク、検査

## 📚 ドキュメント

* [変更履歴](CHANGELOG.md)
* [アーキテクチャと構成のデータ](docs/architecture.json)：2 枚の図を描くためのデータ。

## 🤝 コントリビュートとコミュニティ

* プルリクエストの前に `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`、`python -I tools/ci_validate.py` を実行し、すべて通してください。[CONTRIBUTING.md](CONTRIBUTING.md) と[行動規範](CODE_OF_CONDUCT.md)を参照。
* README は 8 言語あります。文章の変更は 8 ファイルすべてで行い、2 枚の図は `tools/make_architecture.py` と `tools/make_structure.py` が `docs/architecture.json` と `docs/structure.json` から描きます。
* 脆弱性は非公開で報告してください：[SECURITY.md](SECURITY.md)。質問・提案・報告：electrohobby3d@gmail.com（[SUPPORT.md](SUPPORT.md)）。

## 👤 作者

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 ライセンス

ソフトウェア：GPL-3.0-or-later。[LICENSE](LICENSE) を参照。スイートのハードウェア設計には CERN OHL-S を使います。
