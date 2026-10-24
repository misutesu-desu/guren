#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    Tr,
}

impl Lang {
    pub fn detect() -> Self {
        match sys_locale::get_locale() {
            Some(l) if l.to_ascii_lowercase().starts_with("tr") => Lang::Tr,
            _ => Lang::En,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Tr => "tr",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "en" => Some(Lang::En),
            "tr" => Some(Lang::Tr),
            _ => None,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Lang::En => Lang::Tr,
            Lang::Tr => Lang::En,
        }
    }

    pub fn text(self) -> &'static Text {
        match self {
            Lang::En => &EN,
            Lang::Tr => &TR,
        }
    }
}

/// `{name}` and `{error}` in templates are filled in by the caller.
pub struct Text {
    pub no_images: &'static str,
    pub folder_error: &'static str,
    pub open_failed: &'static str,
    pub dialog_title: &'static str,
    pub dialog_filter: &'static str,
    pub open: &'static str,
    pub open_tip: &'static str,
    pub prev_tip: &'static str,
    pub next_tip: &'static str,
    pub fullscreen: &'static str,
    pub fullscreen_tip: &'static str,
    pub leave_fullscreen_tip: &'static str,
    pub theme: &'static str,
    pub theme_system_tip: &'static str,
    pub theme_dark_tip: &'static str,
    pub theme_light_tip: &'static str,
    pub language_tip: &'static str,
    pub actual_size_tip: &'static str,
    pub fit: &'static str,
    pub fit_tip: &'static str,
    pub zoom_in_tip: &'static str,
    pub zoom_out_tip: &'static str,
    pub loading: &'static str,
    pub drop_here: &'static str,
    pub drop_release: &'static str,
    pub or_ctrl_o: &'static str,
    pub open_photo: &'static str,
}

pub const EN: Text = Text {
    no_images: "There are no photos to open here.",
    folder_error: "Could not read the folder: {error}",
    open_failed: "Could not open {name}\n{error}",
    dialog_title: "Open photo",
    dialog_filter: "Photos",
    open: "Open",
    open_tip: "Open (Ctrl+O)",
    prev_tip: "Previous (←)",
    next_tip: "Next (→)",
    fullscreen: "Full screen",
    fullscreen_tip: "Full screen (F11)",
    leave_fullscreen_tip: "Exit full screen (F11)",
    theme: "Theme",
    theme_system_tip: "Theme: system (T)",
    theme_dark_tip: "Theme: dark (T)",
    theme_light_tip: "Theme: light (T)",
    language_tip: "Language (L)",
    actual_size_tip: "Actual size (1)",
    fit: "Fit",
    fit_tip: "Fit to window (0)",
    zoom_in_tip: "Zoom in (+)",
    zoom_out_tip: "Zoom out (−)",
    loading: "loading",
    drop_here: "Drop a photo or a folder here",
    drop_release: "Release to open",
    or_ctrl_o: "or pick one with Ctrl+O",
    open_photo: "Open photo",
};

pub const TR: Text = Text {
    no_images: "Bu konumda açılabilen fotoğraf yok.",
    folder_error: "Klasör okunamadı: {error}",
    open_failed: "{name} açılamadı\n{error}",
    dialog_title: "Fotoğraf aç",
    dialog_filter: "Fotoğraflar",
    open: "Aç",
    open_tip: "Aç (Ctrl+O)",
    prev_tip: "Önceki (←)",
    next_tip: "Sonraki (→)",
    fullscreen: "Tam ekran",
    fullscreen_tip: "Tam ekran (F11)",
    leave_fullscreen_tip: "Tam ekrandan çık (F11)",
    theme: "Tema",
    theme_system_tip: "Tema: sistem (T)",
    theme_dark_tip: "Tema: koyu (T)",
    theme_light_tip: "Tema: açık (T)",
    language_tip: "Dil (L)",
    actual_size_tip: "Gerçek boyut (1)",
    fit: "Sığdır",
    fit_tip: "Pencereye sığdır (0)",
    zoom_in_tip: "Yakınlaştır (+)",
    zoom_out_tip: "Uzaklaştır (−)",
    loading: "yükleniyor",
    drop_here: "Bir fotoğrafı ya da klasörü buraya sürükleyin",
    drop_release: "Bırakınca açılır",
    or_ctrl_o: "ya da Ctrl+O ile seçin",
    open_photo: "Fotoğraf aç",
};
