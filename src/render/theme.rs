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
            "plain" => Self::plain(),
            "amiga" => Self::amiga(),
            _ => Self::default(),
        }
    }

    fn plain() -> Self {
        Self {
            class_background: "#ffffff".into(),
            class_border: "#333333".into(),
            sequence_participant_background: "#ffffff".into(),
            sequence_participant_border: "#333333".into(),
            sequence_lifeline_color: "#888888".into(),
            ..Self::default()
        }
    }

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
            ..Self::default()
        }
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
