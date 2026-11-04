//! The words of the window in the eight languages of the repository (English, Spanish, German, French, Italian, Japanese, Chinese and Portuguese of Portugal).

/// The languages of the window, in the order the lists of words below follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Es,
    De,
    Fr,
    It,
    Ja,
    Zh,
    Pt,
}

impl Lang {
    pub const ALL: [Lang; 8] = [
        Lang::En,
        Lang::Es,
        Lang::De,
        Lang::Fr,
        Lang::It,
        Lang::Ja,
        Lang::Zh,
        Lang::Pt,
    ];

    /// The two letters of the language (`en`, `es`, `de`, `fr`, `it`, `ja`, `zh`, `pt`).
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Es => "es",
            Lang::De => "de",
            Lang::Fr => "fr",
            Lang::It => "it",
            Lang::Ja => "ja",
            Lang::Zh => "zh",
            Lang::Pt => "pt",
        }
    }

    /// The name of the language in the language itself.
    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Es => "Español",
            Lang::De => "Deutsch",
            Lang::Fr => "Français",
            Lang::It => "Italiano",
            Lang::Ja => "日本語",
            Lang::Zh => "中文",
            Lang::Pt => "Português",
        }
    }

    /// The language of a code such as `es`, `es_ES.UTF-8` or `pt-PT`.
    pub fn from_code(code: &str) -> Option<Lang> {
        let two: String = code
            .chars()
            .take(2)
            .collect::<String>()
            .to_ascii_lowercase();
        Lang::ALL.into_iter().find(|l| l.code() == two)
    }

    /// The language of the person from the variables of the system; English when it is none of the eight.
    pub fn from_env() -> Lang {
        for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(v) = std::env::var(var) {
                if !v.is_empty() {
                    return Lang::from_code(&v).unwrap_or(Lang::En);
                }
            }
        }
        Lang::En
    }
}

/// Puts the values in the places `{0}`, `{1}`... of a text.
pub fn fill(template: &str, values: &[&str]) -> String {
    let mut out = template.to_string();
    for (k, v) in values.iter().enumerate() {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

/// Every word and sentence of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Msg {
    Open,
    Export,
    Compute,
    Language,
    Fit,
    DropHint,
    Parameters,
    ToolDiameter,
    Passes,
    Stepover,
    Depth,
    Feed,
    Plunge,
    SafeZ,
    Spindle,
    Resolution,
    Force,
    ZoomHint,
    LegendCopper,
    LegendPaths,
    OpenTitle,
    ExportTitle,
    Folder,
    Up,
    AllFiles,
    Cancel,
    EmptyFolder,
    FileName,
    Ready,
    Opened,
    Computed,
    Exported,
    NoLayer,
    Failed,
    Gaps,
    NotDrawn,
    Stale,
    BadValue,
    SameFile,
}

impl Msg {
    pub const ALL: [Msg; 39] = [
        Msg::Open,
        Msg::Export,
        Msg::Compute,
        Msg::Language,
        Msg::Fit,
        Msg::DropHint,
        Msg::Parameters,
        Msg::ToolDiameter,
        Msg::Passes,
        Msg::Stepover,
        Msg::Depth,
        Msg::Feed,
        Msg::Plunge,
        Msg::SafeZ,
        Msg::Spindle,
        Msg::Resolution,
        Msg::Force,
        Msg::ZoomHint,
        Msg::LegendCopper,
        Msg::LegendPaths,
        Msg::OpenTitle,
        Msg::ExportTitle,
        Msg::Folder,
        Msg::Up,
        Msg::AllFiles,
        Msg::Cancel,
        Msg::EmptyFolder,
        Msg::FileName,
        Msg::Ready,
        Msg::Opened,
        Msg::Computed,
        Msg::Exported,
        Msg::NoLayer,
        Msg::Failed,
        Msg::Gaps,
        Msg::NotDrawn,
        Msg::Stale,
        Msg::BadValue,
        Msg::SameFile,
    ];

    /// The words in the order English, Spanish, German, French, Italian, Japanese, Chinese, Portuguese.
    fn words(self) -> [&'static str; 8] {
        match self {
            Msg::Open => [
                "Open layer...",
                "Abrir capa...",
                "Ebene öffnen...",
                "Ouvrir une couche...",
                "Apri livello...",
                "レイヤーを開く...",
                "打开层...",
                "Abrir camada...",
            ],
            Msg::Export => [
                "Export...",
                "Exportar...",
                "Exportieren...",
                "Exporter...",
                "Esporta...",
                "書き出し...",
                "导出...",
                "Exportar...",
            ],
            Msg::Compute => [
                "Compute paths",
                "Calcular trayectorias",
                "Bahnen berechnen",
                "Calculer les trajets",
                "Calcola i percorsi",
                "パスを計算",
                "计算路径",
                "Calcular trajetórias",
            ],
            Msg::Language => [
                "Language",
                "Idioma",
                "Sprache",
                "Langue",
                "Lingua",
                "言語",
                "语言",
                "Idioma",
            ],
            Msg::Fit => [
                "Fit",
                "Ajustar",
                "Einpassen",
                "Ajuster",
                "Adatta",
                "全体表示",
                "适应窗口",
                "Ajustar",
            ],
            Msg::DropHint => [
                "Drop a Gerber layer here or press Open",
                "Suelte aquí una capa Gerber o pulse Abrir",
                "Eine Gerber-Ebene hierher ziehen oder auf Öffnen klicken",
                "Déposez une couche Gerber ici ou appuyez sur Ouvrir",
                "Trascina qui un livello Gerber o premi Apri",
                "Gerber レイヤーをここにドロップするか「開く」を押してください",
                "将 Gerber 层拖到此处，或点击“打开”",
                "Largue aqui uma camada Gerber ou prima Abrir",
            ],
            Msg::Parameters => [
                "Parameters",
                "Parámetros",
                "Parameter",
                "Paramètres",
                "Parametri",
                "パラメーター",
                "参数",
                "Parâmetros",
            ],
            Msg::ToolDiameter => [
                "Tool diameter (mm)",
                "Diámetro de la fresa (mm)",
                "Werkzeugdurchmesser (mm)",
                "Diamètre de l'outil (mm)",
                "Diametro dell'utensile (mm)",
                "工具径 (mm)",
                "刀具直径 (mm)",
                "Diâmetro da ferramenta (mm)",
            ],
            Msg::Passes => [
                "Passes",
                "Pasadas",
                "Durchgänge",
                "Passes",
                "Passate",
                "パス数",
                "走刀次数",
                "Passagens",
            ],
            Msg::Stepover => [
                "Stepover (mm)",
                "Solape entre pasadas (mm)",
                "Bahnabstand (mm)",
                "Recouvrement (mm)",
                "Sovrapposizione (mm)",
                "ステップオーバー (mm)",
                "步距 (mm)",
                "Sobreposição (mm)",
            ],
            Msg::Depth => [
                "Cutting depth (mm)",
                "Profundidad de corte (mm)",
                "Frästiefe (mm)",
                "Profondeur de coupe (mm)",
                "Profondità di taglio (mm)",
                "切込み深さ (mm)",
                "切削深度 (mm)",
                "Profundidade de corte (mm)",
            ],
            Msg::Feed => [
                "Feed (mm/min)",
                "Avance (mm/min)",
                "Vorschub (mm/min)",
                "Avance (mm/min)",
                "Avanzamento (mm/min)",
                "送り速度 (mm/min)",
                "进给 (mm/min)",
                "Avanço (mm/min)",
            ],
            Msg::Plunge => [
                "Plunge feed (mm/min)",
                "Avance de bajada (mm/min)",
                "Eintauchvorschub (mm/min)",
                "Avance de plongée (mm/min)",
                "Avanzamento in affondo (mm/min)",
                "突っ込み速度 (mm/min)",
                "下刀进给 (mm/min)",
                "Avanço de mergulho (mm/min)",
            ],
            Msg::SafeZ => [
                "Safe height (mm)",
                "Altura segura (mm)",
                "Sichere Höhe (mm)",
                "Hauteur de sécurité (mm)",
                "Altezza di sicurezza (mm)",
                "安全高さ (mm)",
                "安全高度 (mm)",
                "Altura de segurança (mm)",
            ],
            Msg::Spindle => [
                "Spindle (rpm)",
                "Husillo (rpm)",
                "Spindel (U/min)",
                "Broche (tr/min)",
                "Mandrino (giri/min)",
                "スピンドル (rpm)",
                "主轴 (rpm)",
                "Fuso (rpm)",
            ],
            Msg::Resolution => [
                "Grid (mm)",
                "Rejilla (mm)",
                "Raster (mm)",
                "Grille (mm)",
                "Griglia (mm)",
                "グリッド (mm)",
                "网格 (mm)",
                "Grelha (mm)",
            ],
            Msg::Force => [
                "Write even if the tool does not fit",
                "Escribir aunque la fresa no quepa",
                "Auch schreiben, wenn das Werkzeug nicht passt",
                "Écrire même si l'outil ne passe pas",
                "Scrivi anche se l'utensile non entra",
                "工具が入らなくても書き出す",
                "即使刀具放不下也输出",
                "Escrever mesmo que a ferramenta não caiba",
            ],
            Msg::ZoomHint => [
                "Wheel: zoom - drag: move",
                "Rueda: zoom - arrastrar: mover",
                "Rad: Zoom - Ziehen: verschieben",
                "Molette : zoom - glisser : déplacer",
                "Rotella: zoom - trascina: sposta",
                "ホイール: 拡大縮小 - ドラッグ: 移動",
                "滚轮：缩放 - 拖动：移动",
                "Roda: zoom - arrastar: mover",
            ],
            Msg::LegendCopper => [
                "Copper",
                "Cobre",
                "Kupfer",
                "Cuivre",
                "Rame",
                "銅",
                "铜",
                "Cobre",
            ],
            Msg::LegendPaths => [
                "Tool paths",
                "Trayectorias",
                "Werkzeugbahnen",
                "Trajets de l'outil",
                "Percorsi dell'utensile",
                "ツールパス",
                "刀具路径",
                "Trajetórias da ferramenta",
            ],
            Msg::OpenTitle => [
                "Open a Gerber layer",
                "Abrir una capa Gerber",
                "Gerber-Ebene öffnen",
                "Ouvrir une couche Gerber",
                "Apri un livello Gerber",
                "Gerber レイヤーを開く",
                "打开 Gerber 层",
                "Abrir uma camada Gerber",
            ],
            Msg::ExportTitle => [
                "Export the paths (.nc G-code, .plt HP-GL, .dxf)",
                "Exportar las trayectorias (.nc G-code, .plt HP-GL, .dxf)",
                "Bahnen exportieren (.nc G-Code, .plt HP-GL, .dxf)",
                "Exporter les trajets (.nc G-code, .plt HP-GL, .dxf)",
                "Esporta i percorsi (.nc G-code, .plt HP-GL, .dxf)",
                "パスを書き出す (.nc G コード, .plt HP-GL, .dxf)",
                "导出路径 (.nc G 代码, .plt HP-GL, .dxf)",
                "Exportar as trajetórias (.nc G-code, .plt HP-GL, .dxf)",
            ],
            Msg::Folder => [
                "Folder",
                "Carpeta",
                "Ordner",
                "Dossier",
                "Cartella",
                "フォルダー",
                "文件夹",
                "Pasta",
            ],
            Msg::Up => [
                "Up",
                "Subir",
                "Nach oben",
                "Dossier parent",
                "Su",
                "上へ",
                "上一级",
                "Subir",
            ],
            Msg::AllFiles => [
                "All files",
                "Todos los ficheros",
                "Alle Dateien",
                "Tous les fichiers",
                "Tutti i file",
                "すべてのファイル",
                "所有文件",
                "Todos os ficheiros",
            ],
            Msg::Cancel => [
                "Cancel",
                "Cancelar",
                "Abbrechen",
                "Annuler",
                "Annulla",
                "キャンセル",
                "取消",
                "Cancelar",
            ],
            Msg::EmptyFolder => [
                "(nothing to open here)",
                "(nada que abrir aquí)",
                "(hier gibt es nichts zu öffnen)",
                "(rien à ouvrir ici)",
                "(niente da aprire qui)",
                "(ここに開けるものはありません)",
                "(这里没有可打开的内容)",
                "(nada para abrir aqui)",
            ],
            Msg::FileName => [
                "File name",
                "Nombre del fichero",
                "Dateiname",
                "Nom du fichier",
                "Nome del file",
                "ファイル名",
                "文件名",
                "Nome do ficheiro",
            ],
            Msg::Ready => [
                "Ready",
                "Listo",
                "Bereit",
                "Prêt",
                "Pronto",
                "準備完了",
                "就绪",
                "Pronto",
            ],
            Msg::Opened => [
                "{0}: {1} shapes, {2} x {3} mm",
                "{0}: {1} formas, {2} x {3} mm",
                "{0}: {1} Formen, {2} x {3} mm",
                "{0} : {1} formes, {2} x {3} mm",
                "{0}: {1} forme, {2} x {3} mm",
                "{0}: 図形 {1} 個、{2} x {3} mm",
                "{0}：{1} 个图形，{2} x {3} mm",
                "{0}: {1} formas, {2} x {3} mm",
            ],
            Msg::Computed => [
                "{0} passes, {1} paths, {2} mm of cut",
                "{0} pasadas, {1} trayectorias, {2} mm de corte",
                "{0} Durchgänge, {1} Bahnen, {2} mm Schnittlänge",
                "{0} passes, {1} trajets, {2} mm de coupe",
                "{0} passate, {1} percorsi, {2} mm di taglio",
                "{0} パス、{1} 経路、切削長 {2} mm",
                "{0} 次走刀，{1} 条路径，切削长度 {2} mm",
                "{0} passagens, {1} trajetórias, {2} mm de corte",
            ],
            Msg::Exported => [
                "Written to {0}",
                "Escrito en {0}",
                "Geschrieben nach {0}",
                "Écrit dans {0}",
                "Scritto in {0}",
                "{0} に書き出しました",
                "已写入 {0}",
                "Escrito em {0}",
            ],
            Msg::NoLayer => [
                "Open a Gerber layer first",
                "Abra primero una capa Gerber",
                "Zuerst eine Gerber-Ebene öffnen",
                "Ouvrez d'abord une couche Gerber",
                "Apri prima un livello Gerber",
                "先に Gerber レイヤーを開いてください",
                "请先打开一个 Gerber 层",
                "Abra primeiro uma camada Gerber",
            ],
            Msg::Failed => [
                "Error: {0}",
                "Error: {0}",
                "Fehler: {0}",
                "Erreur : {0}",
                "Errore: {0}",
                "エラー: {0}",
                "错误：{0}",
                "Erro: {0}",
            ],
            Msg::Gaps => [
                "The tool does not fit between pieces of copper in {0} places; tick the box to write anyway",
                "La fresa no cabe entre piezas de cobre en {0} sitios; marque la casilla para escribir de todos modos",
                "Das Werkzeug passt an {0} Stellen nicht zwischen die Kupferteile; zum Schreiben trotzdem das Kästchen anhaken",
                "L'outil ne passe pas entre des pièces de cuivre à {0} endroits ; cochez la case pour écrire quand même",
                "L'utensile non entra tra i pezzi di rame in {0} punti; spunta la casella per scrivere comunque",
                "{0} か所で工具が銅の間に入りません。それでも書き出すにはチェックを入れてください",
                "有 {0} 处刀具无法进入铜之间；勾选此框可强制输出",
                "A ferramenta não cabe entre peças de cobre em {0} sítios; assinale a caixa para escrever na mesma",
            ],
            Msg::NotDrawn => [
                "The layer has copper that is not drawn, so the paths would be wrong ({0}); tick the box to write anyway",
                "La capa tiene cobre que no se dibuja, así que las trayectorias serían erróneas ({0}); marque la casilla para escribir de todos modos",
                "Die Ebene enthält Kupfer, das nicht gezeichnet wird; die Bahnen wären falsch ({0}); zum Schreiben trotzdem das Kästchen anhaken",
                "La couche contient du cuivre qui n'est pas dessiné, les trajets seraient faux ({0}) ; cochez la case pour écrire quand même",
                "Il livello ha rame che non viene disegnato, quindi i percorsi sarebbero sbagliati ({0}); spunta la casella per scrivere comunque",
                "描画されない銅があるためパスが正しくありません ({0})。それでも書き出すにはチェックを入れてください",
                "该层有未绘制的铜，路径会出错 ({0})；勾选此框可强制输出",
                "A camada tem cobre que não é desenhado, por isso as trajetórias ficariam erradas ({0}); assinale a caixa para escrever na mesma",
            ],
            Msg::Stale => [
                "The parameters changed: compute the paths again",
                "Los parámetros han cambiado: calcule de nuevo las trayectorias",
                "Die Parameter wurden geändert: Bahnen neu berechnen",
                "Les paramètres ont changé : recalculez les trajets",
                "I parametri sono cambiati: ricalcola i percorsi",
                "パラメーターが変わりました。パスを再計算してください",
                "参数已更改：请重新计算路径",
                "Os parâmetros mudaram: calcule de novo as trajetórias",
            ],
            Msg::BadValue => [
                "{0}: {1}",
                "{0}: {1}",
                "{0}: {1}",
                "{0} : {1}",
                "{0}: {1}",
                "{0}: {1}",
                "{0}：{1}",
                "{0}: {1}",
            ],
            Msg::SameFile => [
                "The output file is the layer itself; choose another name",
                "El fichero de salida es la propia capa; elija otro nombre",
                "Die Ausgabedatei ist die Ebene selbst; einen anderen Namen wählen",
                "Le fichier de sortie est la couche elle-même ; choisissez un autre nom",
                "Il file di uscita è il livello stesso; scegli un altro nome",
                "出力ファイルがレイヤーそのものです。別の名前を選んでください",
                "输出文件就是该层本身；请选择其他名称",
                "O ficheiro de saída é a própria camada; escolha outro nome",
            ],
        }
    }

    /// The text in a language.
    pub fn tr(self, lang: Lang) -> &'static str {
        self.words()[lang as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sentence_has_a_text_in_each_of_the_eight_languages() {
        for m in Msg::ALL {
            for l in Lang::ALL {
                assert!(!m.tr(l).trim().is_empty(), "{m:?} in {}", l.code());
            }
        }
    }

    #[test]
    fn the_list_of_sentences_has_no_repeats() {
        for (i, a) in Msg::ALL.iter().enumerate() {
            assert!(!Msg::ALL[i + 1..].contains(a), "{a:?} is listed twice");
        }
    }

    #[test]
    fn a_template_has_the_same_places_in_every_language() {
        for m in Msg::ALL {
            let places = |l: Lang| {
                (0..4)
                    .filter(|k| m.tr(l).contains(&format!("{{{k}}}")))
                    .count()
            };
            for l in Lang::ALL {
                assert_eq!(places(l), places(Lang::En), "{m:?} in {}", l.code());
            }
        }
    }

    #[test]
    fn the_order_of_the_languages_matches_the_lists() {
        for (k, l) in Lang::ALL.into_iter().enumerate() {
            assert_eq!(l as usize, k);
        }
        assert_eq!(Msg::Cancel.tr(Lang::Es), "Cancelar");
        assert_eq!(Msg::Cancel.tr(Lang::De), "Abbrechen");
        assert_eq!(Msg::Cancel.tr(Lang::Pt), "Cancelar");
    }

    #[test]
    fn a_language_is_found_from_the_code_of_the_system() {
        assert_eq!(Lang::from_code("es_ES.UTF-8"), Some(Lang::Es));
        assert_eq!(Lang::from_code("PT-pt"), Some(Lang::Pt));
        assert_eq!(Lang::from_code("ja"), Some(Lang::Ja));
        assert_eq!(Lang::from_code("xx"), None);
        assert_eq!(Lang::from_code(""), None);
    }

    #[test]
    fn the_places_of_a_template_are_filled() {
        assert_eq!(fill("{1} and {0}", &["a", "b"]), "b and a");
        assert_eq!(fill("none", &["a"]), "none");
    }
}
