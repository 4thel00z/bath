use crate::tui::daisyui_themes;
use crate::tui::daisyui_themes::ColorScheme;
use anyhow::{anyhow, Context, Result};
use ratatui::style::{Color, Style};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BathConfig {
    pub theme: Option<ThemeSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeSection {
    pub preset: Option<String>,

    // Optional overrides (accepts the same strings as daisyUI CSS: e.g. `oklch(...)` or `#RRGGBB`)
    pub base_100: Option<String>,
    pub base_200: Option<String>,
    pub base_300: Option<String>,
    pub base_content: Option<String>,
    pub primary: Option<String>,
    pub primary_content: Option<String>,
    pub secondary: Option<String>,
    pub secondary_content: Option<String>,
    pub accent: Option<String>,
    pub accent_content: Option<String>,
    pub neutral: Option<String>,
    pub neutral_content: Option<String>,
    pub info: Option<String>,
    pub info_content: Option<String>,
    pub success: Option<String>,
    pub success_content: Option<String>,
    pub warning: Option<String>,
    pub warning_content: Option<String>,
    pub error: Option<String>,
    pub error_content: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ThemeColors {
    pub base_100: Color,
    pub base_200: Color,
    pub base_300: Color,
    pub base_content: Color,
    pub primary: Color,
    pub primary_content: Color,
    pub secondary: Color,
    pub secondary_content: Color,
    pub accent: Color,
    pub accent_content: Color,
    pub neutral: Color,
    pub neutral_content: Color,
    pub info: Color,
    pub info_content: Color,
    pub success: Color,
    pub success_content: Color,
    pub warning: Color,
    pub warning_content: Color,
    pub error: Color,
    pub error_content: Color,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub scheme: ColorScheme,
    pub colors: ThemeColors,
}

impl Theme {
    pub fn background(&self) -> Style {
        Style::default().bg(self.colors.base_100)
    }

    pub fn list_highlight(&self) -> Style {
        Style::default()
            .bg(self.colors.primary)
            .fg(self.colors.primary_content)
    }

    pub fn dim_text(&self) -> Style {
        Style::default()
            // Use `neutral` (not `neutral_content`) so it's readable on base backgrounds
            // for both light and dark themes.
            .fg(self.colors.neutral)
            .bg(self.colors.base_100)
    }

    pub fn border(&self) -> Style {
        Style::default()
            // `base_300` can be nearly invisible on light themes; `neutral` reads better.
            .fg(self.colors.neutral)
            .bg(self.colors.base_100)
    }

    pub fn text(&self) -> Style {
        Style::default()
            .fg(self.colors.base_content)
            .bg(self.colors.base_100)
    }
}

pub fn default_preset() -> &'static str {
    "dracula"
}

pub fn load_config() -> Result<BathConfig> {
    let Some(path) = config_path() else {
        return Ok(BathConfig::default());
    };

    let text = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BathConfig::default()),
        Err(e) => return Err(e).with_context(|| format!("read config {}", path.display())),
    };

    toml::from_str::<BathConfig>(&text).with_context(|| format!("parse config {}", path.display()))
}

pub fn save_config(cfg: &BathConfig) -> Result<()> {
    let Some(path) = config_path() else {
        return Ok(());
    };
    let dir = path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;

    let text = toml::to_string_pretty(cfg).context("serialize config")?;
    fs::write(&path, text).with_context(|| format!("write config {}", path.display()))?;
    Ok(())
}

pub fn resolve_from_config(cfg: &BathConfig) -> Result<(Theme, String)> {
    let section = cfg.theme.as_ref();
    let preset = section
        .and_then(|t| t.preset.as_ref())
        .map(|s| s.as_str())
        .unwrap_or(default_preset());
    let theme = resolve_theme(preset, section)?;
    Ok((theme, preset.to_string()))
}

pub fn resolve_theme(preset: &str, overrides: Option<&ThemeSection>) -> Result<Theme> {
    let preset = preset.trim();
    let preset = if preset.is_empty() {
        default_preset()
    } else {
        preset
    };

    let def = daisyui_themes::get(preset)
        .or_else(|| daisyui_themes::get(default_preset()))
        .or_else(|| daisyui_themes::THEMES.first())
        .ok_or_else(|| anyhow!("no daisyUI themes available"))?;

    let default_section = ThemeSection::default();
    let t = overrides.unwrap_or(&default_section);

    let colors = ThemeColors {
        base_100: parse_css_color(t.base_100.as_deref().unwrap_or(def.colors.base_100))
            .context("base_100")?,
        base_200: parse_css_color(t.base_200.as_deref().unwrap_or(def.colors.base_200))
            .context("base_200")?,
        base_300: parse_css_color(t.base_300.as_deref().unwrap_or(def.colors.base_300))
            .context("base_300")?,
        base_content: parse_css_color(t.base_content.as_deref().unwrap_or(def.colors.base_content))
            .context("base_content")?,
        primary: parse_css_color(t.primary.as_deref().unwrap_or(def.colors.primary))
            .context("primary")?,
        primary_content: parse_css_color(
            t.primary_content
                .as_deref()
                .unwrap_or(def.colors.primary_content),
        )
        .context("primary_content")?,
        secondary: parse_css_color(t.secondary.as_deref().unwrap_or(def.colors.secondary))
            .context("secondary")?,
        secondary_content: parse_css_color(
            t.secondary_content
                .as_deref()
                .unwrap_or(def.colors.secondary_content),
        )
        .context("secondary_content")?,
        accent: parse_css_color(t.accent.as_deref().unwrap_or(def.colors.accent))
            .context("accent")?,
        accent_content: parse_css_color(
            t.accent_content
                .as_deref()
                .unwrap_or(def.colors.accent_content),
        )
        .context("accent_content")?,
        neutral: parse_css_color(t.neutral.as_deref().unwrap_or(def.colors.neutral))
            .context("neutral")?,
        neutral_content: parse_css_color(
            t.neutral_content
                .as_deref()
                .unwrap_or(def.colors.neutral_content),
        )
        .context("neutral_content")?,
        info: parse_css_color(t.info.as_deref().unwrap_or(def.colors.info)).context("info")?,
        info_content: parse_css_color(t.info_content.as_deref().unwrap_or(def.colors.info_content))
            .context("info_content")?,
        success: parse_css_color(t.success.as_deref().unwrap_or(def.colors.success))
            .context("success")?,
        success_content: parse_css_color(
            t.success_content
                .as_deref()
                .unwrap_or(def.colors.success_content),
        )
        .context("success_content")?,
        warning: parse_css_color(t.warning.as_deref().unwrap_or(def.colors.warning))
            .context("warning")?,
        warning_content: parse_css_color(
            t.warning_content
                .as_deref()
                .unwrap_or(def.colors.warning_content),
        )
        .context("warning_content")?,
        error: parse_css_color(t.error.as_deref().unwrap_or(def.colors.error)).context("error")?,
        error_content: parse_css_color(
            t.error_content
                .as_deref()
                .unwrap_or(def.colors.error_content),
        )
        .context("error_content")?,
    };

    Ok(Theme {
        name: def.name.to_string(),
        scheme: def.scheme,
        colors,
    })
}

fn config_path() -> Option<PathBuf> {
    let base = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        return None;
    };

    Some(base.join("bath").join("config.toml"))
}

fn parse_css_color(s: &str) -> Result<Color> {
    let s = s.trim();
    if s.is_empty() {
        return Err(anyhow!("empty color"));
    }

    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex_color(hex);
    }

    if let Some(inner) = s.strip_prefix("oklch(").and_then(|v| v.strip_suffix(')')) {
        return parse_oklch(inner);
    }

    Err(anyhow!("unsupported color format: {s}"))
}

fn parse_hex_color(hex: &str) -> Result<Color> {
    let hex = hex.trim();
    let (r, g, b) = match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16)?;
            let g = u8::from_str_radix(&hex[2..4], 16)?;
            let b = u8::from_str_radix(&hex[4..6], 16)?;
            (r, g, b)
        }
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16)?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16)?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16)?;
            (r, g, b)
        }
        _ => return Err(anyhow!("invalid hex color: #{hex}")),
    };
    Ok(Color::Rgb(r, g, b))
}

fn parse_oklch(inner: &str) -> Result<Color> {
    // Format: "<L%> <C> <H>"
    let parts: Vec<&str> = inner.split_whitespace().collect();
    if parts.len() < 3 {
        return Err(anyhow!("invalid oklch(): {inner}"));
    }

    let l_s = parts[0].trim();
    let l = if let Some(p) = l_s.strip_suffix('%') {
        p.parse::<f64>()? / 100.0
    } else {
        l_s.parse::<f64>()?
    };
    let c = parts[1].trim().parse::<f64>()?;
    let h_deg = parts[2].trim().parse::<f64>()?;

    let (r_lin, g_lin, b_lin) = oklch_to_linear_srgb_gamut_mapped(l, c, h_deg)?;
    Ok(Color::Rgb(
        to_u8_srgb(r_lin),
        to_u8_srgb(g_lin),
        to_u8_srgb(b_lin),
    ))
}

fn to_u8_srgb(x: f64) -> u8 {
    let x = x.clamp(0.0, 1.0);
    let srgb = if x <= 0.003_130_8 {
        12.92 * x
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    };
    (srgb.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn oklch_to_linear_srgb_gamut_mapped(l: f64, c: f64, h_deg: f64) -> Result<(f64, f64, f64)> {
    if c <= 0.0 {
        let (r, g, b) = oklab_to_linear_srgb(l, 0.0, 0.0);
        return Ok((r, g, b));
    }

    let (r0, g0, b0) = oklch_to_linear_srgb(l, c, h_deg);
    if in_gamut(r0, g0, b0) {
        return Ok((r0, g0, b0));
    }

    // CSS uses gamut mapping for OKLCH -> sRGB. A simple (and effective) approach is to
    // reduce chroma while keeping L and hue constant until the color is in gamut.
    let mut lo = 0.0;
    let mut hi = c;
    for _ in 0..28 {
        let mid = (lo + hi) / 2.0;
        let (r, g, b) = oklch_to_linear_srgb(l, mid, h_deg);
        if in_gamut(r, g, b) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok(oklch_to_linear_srgb(l, lo, h_deg))
}

fn oklch_to_linear_srgb(l: f64, c: f64, h_deg: f64) -> (f64, f64, f64) {
    let h = h_deg.to_radians();
    let a = c * h.cos();
    let b = c * h.sin();
    oklab_to_linear_srgb(l, a, b)
}

fn oklab_to_linear_srgb(l: f64, a: f64, b: f64) -> (f64, f64, f64) {
    // OKLab -> linear sRGB (Björn Ottosson)
    let l_ = l + 0.396_337_777_4 * a + 0.215_803_757_3 * b;
    let m_ = l - 0.105_561_345_8 * a - 0.063_854_172_8 * b;
    let s_ = l - 0.089_484_177_5 * a - 1.291_485_548_0 * b;

    let l3 = l_ * l_ * l_;
    let m3 = m_ * m_ * m_;
    let s3 = s_ * s_ * s_;

    let r_lin = 4.076_741_662_1 * l3 - 3.307_711_591_3 * m3 + 0.230_969_929_2 * s3;
    let g_lin = -1.268_438_004_6 * l3 + 2.609_757_401_1 * m3 - 0.341_319_396_5 * s3;
    let b_lin = -0.004_196_086_3 * l3 - 0.703_418_614_7 * m3 + 1.707_614_701_0 * s3;

    (r_lin, g_lin, b_lin)
}

fn in_gamut(r: f64, g: f64, b: f64) -> bool {
    (0.0..=1.0).contains(&r) && (0.0..=1.0).contains(&g) && (0.0..=1.0).contains(&b)
}
