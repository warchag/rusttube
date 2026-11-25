use crate::models::{AccentColor, AppTheme};
use eframe::egui::{self, Color32, Rounding, Stroke, Vec2};

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg_main: Color32,
    pub bg_sidebar: Color32,
    pub bg_card: Color32,
    pub bg_card_alt: Color32,
    pub bg_hover: Color32,
    pub border: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub green: Color32,
    pub blue: Color32,
    pub amber: Color32,
    pub red: Color32,
    pub red_soft: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
}

impl Palette {
    pub fn from_config(theme: AppTheme, accent_choice: AccentColor) -> Self {
        let mut pal = match theme {
            AppTheme::MidnightStudio => Self {
                bg_main: Color32::from_rgb(14, 15, 22),
                bg_sidebar: Color32::from_rgb(19, 21, 30),
                bg_card: Color32::from_rgb(25, 27, 38),
                bg_card_alt: Color32::from_rgb(33, 36, 50),
                bg_hover: Color32::from_rgb(44, 48, 66),
                border: Color32::from_rgb(48, 52, 70),
                accent: Color32::from_rgb(255, 51, 88),
                accent_soft: Color32::from_rgb(64, 24, 34),
                green: Color32::from_rgb(46, 204, 113),
                blue: Color32::from_rgb(77, 159, 255),
                amber: Color32::from_rgb(255, 184, 77),
                red: Color32::from_rgb(255, 71, 87),
                red_soft: Color32::from_rgb(64, 24, 34),
                text_primary: Color32::from_rgb(242, 244, 250),
                text_secondary: Color32::from_rgb(155, 162, 184),
                text_muted: Color32::from_rgb(106, 113, 134),
            },
            AppTheme::TokyoNight => Self {
                bg_main: Color32::from_rgb(22, 22, 30),
                bg_sidebar: Color32::from_rgb(26, 27, 38),
                bg_card: Color32::from_rgb(31, 35, 53),
                bg_card_alt: Color32::from_rgb(41, 46, 66),
                bg_hover: Color32::from_rgb(54, 61, 88),
                border: Color32::from_rgb(65, 72, 104),
                accent: Color32::from_rgb(122, 162, 247),
                accent_soft: Color32::from_rgb(36, 48, 78),
                green: Color32::from_rgb(158, 206, 106),
                blue: Color32::from_rgb(125, 207, 255),
                amber: Color32::from_rgb(224, 175, 104),
                red: Color32::from_rgb(247, 118, 142),
                red_soft: Color32::from_rgb(66, 30, 44),
                text_primary: Color32::from_rgb(192, 202, 245),
                text_secondary: Color32::from_rgb(154, 165, 206),
                text_muted: Color32::from_rgb(86, 95, 137),
            },
            AppTheme::CatppuccinMocha => Self {
                bg_main: Color32::from_rgb(24, 24, 37),
                bg_sidebar: Color32::from_rgb(30, 30, 46),
                bg_card: Color32::from_rgb(39, 39, 58),
                bg_card_alt: Color32::from_rgb(49, 50, 68),
                bg_hover: Color32::from_rgb(69, 71, 90),
                border: Color32::from_rgb(76, 79, 105),
                accent: Color32::from_rgb(203, 166, 247),
                accent_soft: Color32::from_rgb(58, 45, 78),
                green: Color32::from_rgb(166, 227, 161),
                blue: Color32::from_rgb(137, 180, 250),
                amber: Color32::from_rgb(250, 179, 135),
                red: Color32::from_rgb(243, 139, 168),
                red_soft: Color32::from_rgb(68, 34, 48),
                text_primary: Color32::from_rgb(205, 214, 244),
                text_secondary: Color32::from_rgb(166, 173, 200),
                text_muted: Color32::from_rgb(127, 132, 156),
            },
            AppTheme::CyberpunkNeon => Self {
                bg_main: Color32::from_rgb(10, 9, 20),
                bg_sidebar: Color32::from_rgb(16, 13, 32),
                bg_card: Color32::from_rgb(22, 18, 42),
                bg_card_alt: Color32::from_rgb(32, 26, 58),
                bg_hover: Color32::from_rgb(46, 36, 78),
                border: Color32::from_rgb(84, 44, 118),
                accent: Color32::from_rgb(255, 42, 133),
                accent_soft: Color32::from_rgb(74, 18, 52),
                green: Color32::from_rgb(0, 245, 160),
                blue: Color32::from_rgb(0, 217, 245),
                amber: Color32::from_rgb(255, 209, 102),
                red: Color32::from_rgb(255, 62, 108),
                red_soft: Color32::from_rgb(74, 18, 40),
                text_primary: Color32::from_rgb(245, 240, 255),
                text_secondary: Color32::from_rgb(180, 165, 214),
                text_muted: Color32::from_rgb(116, 102, 154),
            },
            AppTheme::NordicFrost => Self {
                bg_main: Color32::from_rgb(36, 41, 51),
                bg_sidebar: Color32::from_rgb(46, 52, 64),
                bg_card: Color32::from_rgb(53, 60, 74),
                bg_card_alt: Color32::from_rgb(67, 76, 94),
                bg_hover: Color32::from_rgb(76, 86, 106),
                border: Color32::from_rgb(84, 96, 118),
                accent: Color32::from_rgb(136, 192, 208),
                accent_soft: Color32::from_rgb(44, 68, 80),
                green: Color32::from_rgb(163, 190, 140),
                blue: Color32::from_rgb(129, 161, 193),
                amber: Color32::from_rgb(235, 203, 139),
                red: Color32::from_rgb(191, 97, 106),
                red_soft: Color32::from_rgb(68, 40, 46),
                text_primary: Color32::from_rgb(236, 239, 244),
                text_secondary: Color32::from_rgb(216, 222, 233),
                text_muted: Color32::from_rgb(144, 156, 178),
            },
            AppTheme::Dracula => Self {
                bg_main: Color32::from_rgb(30, 31, 41),
                bg_sidebar: Color32::from_rgb(40, 42, 54),
                bg_card: Color32::from_rgb(49, 52, 68),
                bg_card_alt: Color32::from_rgb(62, 66, 86),
                bg_hover: Color32::from_rgb(74, 79, 104),
                border: Color32::from_rgb(86, 92, 122),
                accent: Color32::from_rgb(189, 147, 249),
                accent_soft: Color32::from_rgb(64, 48, 92),
                green: Color32::from_rgb(80, 250, 123),
                blue: Color32::from_rgb(139, 233, 253),
                amber: Color32::from_rgb(255, 184, 108),
                red: Color32::from_rgb(255, 85, 85),
                red_soft: Color32::from_rgb(76, 32, 40),
                text_primary: Color32::from_rgb(248, 248, 242),
                text_secondary: Color32::from_rgb(198, 202, 224),
                text_muted: Color32::from_rgb(132, 140, 176),
            },
            AppTheme::SakuraBlossom => Self {
                bg_main: Color32::from_rgb(255, 245, 248),
                bg_sidebar: Color32::from_rgb(255, 251, 252),
                bg_card: Color32::from_rgb(255, 255, 255),
                bg_card_alt: Color32::from_rgb(252, 235, 241),
                bg_hover: Color32::from_rgb(248, 220, 230),
                border: Color32::from_rgb(240, 204, 218),
                accent: Color32::from_rgb(236, 72, 128),
                accent_soft: Color32::from_rgb(255, 226, 238),
                green: Color32::from_rgb(22, 163, 74),
                blue: Color32::from_rgb(59, 130, 246),
                amber: Color32::from_rgb(217, 119, 6),
                red: Color32::from_rgb(225, 29, 72),
                red_soft: Color32::from_rgb(255, 226, 234),
                text_primary: Color32::from_rgb(56, 28, 42),
                text_secondary: Color32::from_rgb(122, 76, 98),
                text_muted: Color32::from_rgb(166, 122, 142),
            },
            AppTheme::AppleSnow => Self {
                bg_main: Color32::from_rgb(244, 246, 251),
                bg_sidebar: Color32::from_rgb(255, 255, 255),
                bg_card: Color32::from_rgb(255, 255, 255),
                bg_card_alt: Color32::from_rgb(236, 240, 248),
                bg_hover: Color32::from_rgb(225, 230, 242),
                border: Color32::from_rgb(214, 220, 234),
                accent: Color32::from_rgb(0, 122, 255),
                accent_soft: Color32::from_rgb(226, 240, 255),
                green: Color32::from_rgb(22, 163, 74),
                blue: Color32::from_rgb(37, 99, 235),
                amber: Color32::from_rgb(217, 119, 6),
                red: Color32::from_rgb(225, 29, 72),
                red_soft: Color32::from_rgb(255, 230, 236),
                text_primary: Color32::from_rgb(22, 26, 38),
                text_secondary: Color32::from_rgb(82, 92, 116),
                text_muted: Color32::from_rgb(126, 136, 160),
            },
        };

        // Optional user accent override
        let is_light = theme.is_light();
        let override_accent = match accent_choice {
            AccentColor::ThemeDefault => None,
            AccentColor::Red => Some((
                Color32::from_rgb(255, 51, 88),
                Color32::from_rgb(64, 24, 34),
                Color32::from_rgb(255, 232, 237),
            )),
            AccentColor::Purple => Some((
                Color32::from_rgb(147, 99, 255),
                Color32::from_rgb(46, 30, 74),
                Color32::from_rgb(240, 233, 255),
            )),
            AccentColor::Blue => Some((
                Color32::from_rgb(59, 142, 255),
                Color32::from_rgb(24, 42, 72),
                Color32::from_rgb(230, 242, 255),
            )),
            AccentColor::Emerald => Some((
                Color32::from_rgb(32, 196, 128),
                Color32::from_rgb(20, 56, 42),
                Color32::from_rgb(228, 250, 240),
            )),
            AccentColor::Amber => Some((
                Color32::from_rgb(245, 158, 11),
                Color32::from_rgb(62, 44, 18),
                Color32::from_rgb(255, 244, 224),
            )),
            AccentColor::Pink => Some((
                Color32::from_rgb(244, 63, 152),
                Color32::from_rgb(68, 22, 48),
                Color32::from_rgb(255, 228, 242),
            )),
        };

        if let Some((a, soft_d, soft_l)) = override_accent {
            pal.accent = a;
            pal.accent_soft = if is_light { soft_l } else { soft_d };
        }

        pal
    }
}

pub fn apply_theme(ctx: &egui::Context, theme: AppTheme, pal: &Palette) {
    let mut visuals = if theme.is_light() {
        egui::Visuals::light()
    } else {
        egui::Visuals::dark()
    };

    visuals.window_fill = pal.bg_main;
    visuals.panel_fill = pal.bg_main;
    visuals.faint_bg_color = pal.bg_card;
    visuals.extreme_bg_color = pal.bg_card_alt;
    visuals.selection.bg_fill = pal.accent.gamma_multiply(0.35);
    visuals.selection.stroke = Stroke::new(1.0, pal.accent);

    visuals.widgets.noninteractive.bg_fill = pal.bg_card;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, pal.text_primary);
    visuals.widgets.noninteractive.rounding = Rounding::same(8.0);

    visuals.widgets.inactive.bg_fill = pal.bg_card_alt;
    visuals.widgets.inactive.weak_bg_fill = pal.bg_card_alt;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, pal.border);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, pal.text_primary);
    visuals.widgets.inactive.rounding = Rounding::same(8.0);

    visuals.widgets.hovered.bg_fill = pal.bg_hover;
    visuals.widgets.hovered.weak_bg_fill = pal.bg_hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, pal.accent.gamma_multiply(0.75));
    visuals.widgets.hovered.rounding = Rounding::same(8.0);

    visuals.widgets.active.bg_fill = pal.accent;
    visuals.widgets.active.rounding = Rounding::same(8.0);

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(10.0, 6.0);
    ctx.set_style(style);
}

/// Configure crisp macOS system font + Thai Unicode fallback font so Thai titles render cleanly.
pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    if let Ok(bytes) = std::fs::read("/System/Library/Fonts/SFNS.ttf") {
        fonts
            .font_data
            .insert("sf_pro".to_owned(), egui::FontData::from_owned(bytes));
        if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            fam.insert(0, "sf_pro".to_owned());
        }
    }

    let thai_candidates = [
        "/System/Library/Fonts/Supplemental/Sathu.ttf",
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/System/Library/Fonts/ThonburiUI.ttc",
    ];
    for (idx, path) in thai_candidates.iter().enumerate() {
        if let Ok(bytes) = std::fs::read(path) {
            let key = format!("thai_fallback_{idx}");
            fonts
                .font_data
                .insert(key.clone(), egui::FontData::from_owned(bytes));
            if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                fam.push(key.clone());
            }
            if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                fam.push(key);
            }
        }
    }

    ctx.set_fonts(fonts);
}
