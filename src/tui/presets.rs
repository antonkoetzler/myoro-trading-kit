//! Predefined color presets and palette constructors for the theme creator.

use crate::tui::theme::ThemePalette;

pub(super) fn default_dark() -> ThemePalette {
    ThemePalette {
        name: "Default Dark".into(),
        bg: [8, 8, 10],
        panel_bg: [16, 16, 18],
        border: [72, 70, 58],
        heading: [180, 172, 130],
        body: [165, 162, 155],
        dim: [100, 98, 92],
        accent: [255, 220, 100],
        success: [140, 180, 120],
        key: [170, 150, 210],
        danger: [220, 120, 100],
        neutral_pnl: [200, 180, 100],
    }
}

pub(super) fn nord() -> ThemePalette {
    ThemePalette {
        name: "Nord".into(),
        bg: [46, 52, 64],
        panel_bg: [59, 66, 82],
        border: [76, 86, 106],
        heading: [143, 188, 187],
        body: [200, 205, 215],
        dim: [129, 161, 193],
        accent: [136, 192, 208],
        success: [163, 190, 140],
        key: [180, 142, 173],
        danger: [191, 97, 106],
        neutral_pnl: [235, 203, 139],
    }
}

pub(super) fn dracula() -> ThemePalette {
    ThemePalette {
        name: "Dracula".into(),
        bg: [40, 42, 54],
        panel_bg: [49, 51, 68],
        border: [68, 71, 90],
        heading: [189, 147, 249],
        body: [210, 210, 200],
        dim: [98, 114, 164],
        accent: [255, 121, 198],
        success: [80, 250, 123],
        key: [255, 184, 108],
        danger: [255, 85, 85],
        neutral_pnl: [241, 250, 140],
    }
}

pub(super) fn green() -> ThemePalette {
    ThemePalette {
        name: "Green Terminal".into(),
        bg: [0, 20, 0],
        panel_bg: [0, 35, 0],
        border: [40, 80, 40],
        heading: [150, 255, 150],
        body: [180, 255, 180],
        dim: [80, 140, 80],
        accent: [200, 255, 200],
        success: [100, 255, 100],
        key: [150, 255, 200],
        danger: [255, 100, 100],
        neutral_pnl: [200, 255, 150],
    }
}

pub(super) fn monokai() -> ThemePalette {
    ThemePalette {
        name: "Monokai".into(),
        bg: [39, 40, 34],
        panel_bg: [49, 51, 53],
        border: [73, 72, 62],
        heading: [249, 38, 114],
        body: [215, 215, 208],
        dim: [117, 113, 94],
        accent: [102, 217, 239],
        success: [166, 226, 46],
        key: [230, 219, 116],
        danger: [249, 38, 114],
        neutral_pnl: [230, 219, 116],
    }
}

pub fn predefined_themes() -> Vec<ThemePalette> {
    vec![default_dark(), nord(), dracula(), green(), monokai()]
}

/// Preset colors for theme creator (indexable).
pub const COLOR_PRESETS: [[u8; 3]; 24] = [
    [0, 0, 0],
    [8, 8, 10],
    [46, 52, 64],
    [40, 42, 54],
    [255, 255, 255],
    [200, 198, 190],
    [248, 248, 242],
    [236, 239, 244],
    [255, 220, 100],
    [136, 192, 208],
    [189, 147, 249],
    [102, 217, 239],
    [140, 180, 120],
    [80, 250, 123],
    [166, 226, 46],
    [220, 120, 100],
    [249, 38, 114],
    [191, 97, 106],
    [180, 160, 220],
    [255, 184, 108],
    [120, 115, 100],
    [117, 113, 94],
    [72, 70, 58],
    [76, 86, 106],
];

pub const THEME_CREATOR_ROLES: &[&str] = &[
    "Background",
    "Panel BG",
    "Border",
    "Heading",
    "Body",
    "Dim",
    "Accent",
    "Success",
    "Key",
    "Danger",
    "Neutral P&L",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predefined_themes_has_five_entries() {
        let themes = predefined_themes();
        assert_eq!(themes.len(), 5);
    }

    #[test]
    fn color_presets_has_24_entries() {
        assert_eq!(COLOR_PRESETS.len(), 24);
    }

    #[test]
    fn theme_creator_roles_has_11_entries() {
        assert_eq!(THEME_CREATOR_ROLES.len(), 11);
    }
}
