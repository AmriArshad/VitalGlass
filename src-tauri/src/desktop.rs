//! Desktop integration: read the Plasma colour scheme so the "System"
//! theme follows KDE's light/dark choice and accent colour.
//!
//! Plasma writes the active scheme to `~/.config/kdeglobals` as
//! `r,g,b` triples. Outside KDE the file is missing and the frontend falls
//! back to the `prefers-color-scheme` media query and Breeze blue.

use std::collections::HashMap;

use serde::Serialize;

#[derive(Debug, Default, Serialize)]
pub struct DesktopTheme {
    /// `#rrggbb`, from the user's accent colour or the scheme's selection colour.
    pub accent: Option<String>,
    /// Whether the active colour scheme is dark (window background luminance).
    pub dark: Option<bool>,
}

/// Read the current Plasma colour scheme. Cheap enough to call whenever the
/// window regains focus, which is how theme changes get picked up.
#[tauri::command]
pub fn desktop_theme() -> DesktopTheme {
    let Some(path) = dirs::config_dir().map(|d| d.join("kdeglobals")) else {
        return DesktopTheme::default();
    };
    match std::fs::read_to_string(path) {
        Ok(text) => parse(&text),
        Err(_) => DesktopTheme::default(),
    }
}

fn parse(text: &str) -> DesktopTheme {
    let ini = sections(text);
    let get = |section: &str, key: &str| ini.get(section).and_then(|s| s.get(key)).and_then(|v| rgb(v));
    let accent = get("General", "AccentColor").or_else(|| get("Colors:Selection", "BackgroundNormal"));
    let window = get("Colors:Window", "BackgroundNormal");
    DesktopTheme {
        accent: accent.map(|[r, g, b]| format!("#{r:02x}{g:02x}{b:02x}")),
        dark: window.map(|c| luminance(c) < 0.5),
    }
}

/// Minimal INI reader: `[Section]` headers and `key=value` lines.
fn sections(text: &str) -> HashMap<&str, HashMap<&str, &str>> {
    let mut out: HashMap<&str, HashMap<&str, &str>> = HashMap::new();
    let mut current = "";
    for line in text.lines().map(str::trim) {
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            current = name;
        } else if let Some((k, v)) = line.split_once('=') {
            out.entry(current).or_default().insert(k.trim(), v.trim());
        }
    }
    out
}

/// `"61,174,233"` -> `[61, 174, 233]`
fn rgb(s: &str) -> Option<[u8; 3]> {
    let mut it = s.split(',').map(|p| p.trim().parse::<u8>().ok());
    let c = [it.next()??, it.next()??, it.next()??];
    Some(c)
}

/// Perceived brightness, 0 (black) to 1 (white).
fn luminance([r, g, b]: [u8; 3]) -> f64 {
    (0.299 * f64::from(r) + 0.587 * f64::from(g) + 0.114 * f64::from(b)) / 255.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_breeze_dark() {
        let t = parse(
            "[Colors:Selection]\nBackgroundNormal=61,174,233\n\n[Colors:Window]\nBackgroundNormal=32,35,38\n[General]\nColorSchemeHash=abc\n",
        );
        assert_eq!(t.accent.as_deref(), Some("#3daee9"));
        assert_eq!(t.dark, Some(true));
    }

    #[test]
    fn custom_accent_wins() {
        let t = parse("[General]\nAccentColor=233,100,61\n[Colors:Selection]\nBackgroundNormal=61,174,233\n");
        assert_eq!(t.accent.as_deref(), Some("#e9643d"));
        assert_eq!(t.dark, None);
    }
}
