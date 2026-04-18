/// Light vs. dark handling. `Auto` emits a single SVG whose palette follows
/// the viewer's `prefers-color-scheme` via CSS media queries; `Light`/`Dark`
/// bake a single palette into the output and don't adapt.
///
/// `Auto` is the default — a bare `.puml` with no `!theme` directive gets
/// an SVG that renders readable in light viewers and flips automatically
/// in dark ones. Users who want a strictly static palette opt in with
/// `!theme light` / `!theme dark` or `--theme light` / `--theme dark`.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ColorScheme {
    #[default]
    Auto,
    Light,
    Dark,
}

/// Resolved visual parameters for rendering.
///
/// Every renderer reads from this struct instead of using hard-coded colours
/// and fonts. The defaults reproduce the pre-theme values exactly so unrelated
/// snapshots stay stable — user-supplied `skinparam` lines or a `!theme`
/// directive override selected fields.
#[derive(Debug, Clone)]
pub struct Theme {
    pub background_color: String,
    pub font_family: String,
    pub font_size: f64,
    pub font_color: String,

    pub arrow_color: String,
    pub arrow_thickness: f64,

    pub class_background: String,
    pub class_border: String,

    pub sequence_participant_background: String,
    pub sequence_participant_border: String,
    pub sequence_lifeline_color: String,

    pub note_background: String,
    pub note_border: String,

    /// When `Auto`, the rendered SVG toggles its palette based on the
    /// viewer's `prefers-color-scheme`. `Light` and `Dark` bake a single
    /// palette into the output.
    pub color_scheme: ColorScheme,
    /// The dark-mode counterparts for `Auto` — only consulted when
    /// `color_scheme` is `Auto`. Falls back to the dark preset's defaults.
    pub dark_background_color: String,
    pub dark_font_color: String,
    pub dark_arrow_color: String,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            background_color: "#ffffff".into(),
            font_family: "\"Liberation Sans\",Helvetica,Arial,sans-serif".into(),
            font_size: 13.0,
            font_color: "#181818".into(),

            arrow_color: "#181818".into(),
            arrow_thickness: 1.5,

            class_background: "#dae8fc".into(),
            class_border: "#6c8ebf".into(),

            sequence_participant_background: "#dae8fc".into(),
            sequence_participant_border: "#6c8ebf".into(),
            sequence_lifeline_color: "#6c8ebf".into(),

            note_background: "#ffffc0".into(),
            note_border: "#bbbb00".into(),

            color_scheme: ColorScheme::default(),
            dark_background_color: "#1e1e1e".into(),
            dark_font_color: "#e8e8e8".into(),
            dark_arrow_color: "#e8e8e8".into(),
        }
    }
}

impl Theme {
    /// Apply a single `skinparam key value` pair. Unknown keys are ignored
    /// (per PlantUML compatibility — unknown skinparams warn, never fail).
    pub fn apply_skinparam(&mut self, key: &str, value: &str) {
        let value = value.trim();
        if value.is_empty() {
            return;
        }
        match normalize_key(key).as_str() {
            // `!theme foo` is translated to `skinparam theme foo` by the
            // preprocessor. Applying it reloads the preset, which then gets
            // overwritten by any subsequent skinparam lines.
            "theme" => *self = Self::from_preset(value),
            "backgroundcolor" => self.background_color = value.to_string(),
            "defaultfontname" | "fontname" => self.font_family = unquote(value).to_string(),
            "defaultfontsize" | "fontsize" => {
                if let Ok(n) = value.parse::<f64>() {
                    self.font_size = n;
                }
            }
            "defaultfontcolor" | "fontcolor" => self.font_color = value.to_string(),

            "arrowcolor" | "sequencearrowcolor" | "classarrowcolor" => {
                self.arrow_color = value.to_string();
            }
            "arrowthickness" | "sequencearrowthickness" => {
                if let Ok(n) = value.parse::<f64>() {
                    self.arrow_thickness = n;
                }
            }

            "classbackgroundcolor" => self.class_background = value.to_string(),
            "classbordercolor" => self.class_border = value.to_string(),

            "participantbackgroundcolor" | "sequenceparticipantbackgroundcolor" => {
                self.sequence_participant_background = value.to_string();
            }
            "participantbordercolor" | "sequenceparticipantbordercolor" => {
                self.sequence_participant_border = value.to_string();
            }
            "lifelinestrokecolor" | "sequencelifelinecolor" => {
                self.sequence_lifeline_color = value.to_string();
            }

            "notebackgroundcolor" => self.note_background = value.to_string(),
            "notebordercolor" => self.note_border = value.to_string(),

            _ => {}
        }
    }

    /// Apply every `(key, value)` pair from an AST's collected skinparams.
    pub fn apply_all<'a, I>(&mut self, pairs: I)
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        for (k, v) in pairs {
            self.apply_skinparam(k, v);
        }
    }

    /// Select a built-in theme preset. Unknown names fall back to default.
    pub fn from_preset(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "light" => Self::light(),
            "plain" => Self::plain(),
            "amiga" => Self::amiga(),
            "dark" => Self::dark(),
            "auto" | "adaptive" => Self::auto(),
            _ => Self::default(),
        }
    }

    /// Static light palette — explicit opt-out of the adaptive default.
    /// Matches the classic hard-coded values used before theming existed.
    fn light() -> Self {
        Self {
            color_scheme: ColorScheme::Light,
            ..Self::default()
        }
    }

    /// Monochrome white/grey palette. Pinned to Light because a white
    /// background would invert awkwardly in a dark viewer.
    fn plain() -> Self {
        Self {
            class_background: "#ffffff".into(),
            class_border: "#333333".into(),
            sequence_participant_background: "#ffffff".into(),
            sequence_participant_border: "#333333".into(),
            sequence_lifeline_color: "#888888".into(),
            color_scheme: ColorScheme::Light,
            ..Self::default()
        }
    }

    /// Retro Amiga deep-blue palette. Pinned to Light (i.e. static) because
    /// the custom colours are the whole point — flipping them to a generic
    /// dark palette under `prefers-color-scheme: dark` would defeat the
    /// theme.
    fn amiga() -> Self {
        Self {
            background_color: "#000088".into(),
            font_color: "#ffffff".into(),
            arrow_color: "#ffffff".into(),
            class_background: "#000088".into(),
            class_border: "#ff8800".into(),
            sequence_participant_background: "#000088".into(),
            sequence_participant_border: "#ff8800".into(),
            sequence_lifeline_color: "#ff8800".into(),
            note_background: "#ffaa00".into(),
            note_border: "#ff8800".into(),
            color_scheme: ColorScheme::Light,
            ..Self::default()
        }
    }

    /// Explicit dark palette. Backgrounds shift near-black, text near-white,
    /// shape fills and borders pulled toward muted cool tones that retain
    /// enough contrast against the dark canvas.
    fn dark() -> Self {
        Self {
            background_color: "#1e1e1e".into(),
            font_color: "#e8e8e8".into(),
            arrow_color: "#d0d0d0".into(),
            class_background: "#2d3748".into(),
            class_border: "#7fa0c8".into(),
            sequence_participant_background: "#2d3748".into(),
            sequence_participant_border: "#7fa0c8".into(),
            sequence_lifeline_color: "#888888".into(),
            note_background: "#3a3a00".into(),
            note_border: "#bbbb55".into(),
            color_scheme: ColorScheme::Dark,
            ..Self::default()
        }
    }

    /// Adaptive palette. Default light palette drives the base SVG, and a
    /// CSS `@media (prefers-color-scheme: dark)` rule swaps background +
    /// text colours when the viewer prefers dark. Everything else (shape
    /// fills, borders) stays constant — the palette is already tuned for
    /// legibility on both backgrounds. This is the default scheme; calling
    /// `Self::auto()` is equivalent to `Self::default()`, kept for clarity
    /// at call sites that want to name the preset.
    fn auto() -> Self {
        Self::default()
    }
}

fn normalize_key(k: &str) -> String {
    k.trim().to_lowercase()
}

fn unquote(s: &str) -> &str {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_preserves_baseline_colours() {
        let t = Theme::default();
        assert_eq!(t.background_color, "#ffffff");
        assert_eq!(t.class_background, "#dae8fc");
    }

    #[test]
    fn apply_skinparam_background() {
        let mut t = Theme::default();
        t.apply_skinparam("backgroundColor", "#f0f0f0");
        assert_eq!(t.background_color, "#f0f0f0");
    }

    #[test]
    fn apply_skinparam_case_insensitive() {
        let mut t = Theme::default();
        t.apply_skinparam("BACKGROUNDCOLOR", "#ff0000");
        assert_eq!(t.background_color, "#ff0000");
    }

    #[test]
    fn font_size_parses_numerically() {
        let mut t = Theme::default();
        t.apply_skinparam("DefaultFontSize", "18");
        assert_eq!(t.font_size, 18.0);
    }

    #[test]
    fn unknown_skinparam_ignored() {
        let mut t = Theme::default();
        t.apply_skinparam("bogusParam", "anything");
        // No change, no panic
        assert_eq!(t.background_color, "#ffffff");
    }

    #[test]
    fn preset_amiga_overrides_default() {
        let t = Theme::from_preset("amiga");
        assert_eq!(t.background_color, "#000088");
    }
}
