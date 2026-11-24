//! Rich text document independent of controls and UI thread.
use super::richtext::TextFormat;
use crate::utilities::Color;

/// Builds formatted spans without RTF knowledge. Can be produced in the background and displayed through
/// [`super::RichTextBox::set_rtf`].
#[derive(Clone, Debug)]
pub struct StyledText {
    family: String,
    points: f32,
    runs: Vec<(String, TextFormat)>,
}

impl StyledText {
    /// Creates a document with default font and point size.
    pub fn new(family: &str, points: f32) -> Self {
        Self { family: family.into(), points, runs: Vec::new() }
    }

    /// Appends literal text with formatting independent of the preceding span.
    pub fn push(&mut self, text: &str, format: TextFormat) {
        self.runs.push((text.into(), format));
    }

    /// Appends text formatted by Unicode character position. Callback receives scalar index and character without
    /// byte/UTF-16 conversion. Adjacent identical styles are merged.
    pub fn push_mapped(&mut self, text: &str, mut style: impl FnMut(usize, char) -> TextFormat) {
        let mut run = String::new();
        let mut previous = None;
        for (index, ch) in text.chars().enumerate() {
            let format = style(index, ch);
            if previous.as_ref().is_some_and(|p| p != &format) {
                if let Some(format) = previous.take() {
                    self.push(&run, format);
                }
                run.clear();
            }
            run.push(ch);
            previous = Some(format);
        }
        if let Some(format) = previous {
            self.push(&run, format);
        }
    }

    /// Serializes Unicode RTF, escaping control characters. Normalizes CRLF/CR/LF; colors retain RGB (RTF has no
    /// alpha). Invalid sizes normalize to 10 points.
    pub fn to_rtf(&self) -> String {
        let mut fonts = vec![self.family.clone()];
        let mut colors: Vec<Color> = Vec::new();
        for (_, f) in &self.runs {
            if let Some(name) = &f.family {
                if !fonts.contains(name) {
                    fonts.push(name.clone());
                }
            }
            for color in [f.color, f.highlight].into_iter().flatten() {
                if !colors.contains(&color) {
                    colors.push(color);
                }
            }
        }
        let mut out = String::from("{\\rtf1\\ansi\\deff0\\uc1{\\fonttbl");
        for (i, name) in fonts.iter().enumerate() {
            out.push_str(&format!("{{\\f{i} "));
            literal(&name.replace(';', ""), &mut out);
            out.push_str(";}");
        }
        out.push_str("}{\\colortbl;");
        for c in &colors {
            out.push_str(&format!("\\red{}\\green{}\\blue{};", c.r, c.g, c.b));
        }
        out.push('}');
        for (text, f) in &self.runs {
            let font = f.family.as_ref().and_then(|name| fonts.iter().position(|n| n == name)).unwrap_or(0);
            let color = |c: Option<Color>| c.and_then(|c| colors.iter().position(|x| *x == c)).map_or(0, |i| i + 1);
            out.push_str(&format!(
                "{{\\plain\\f{font}\\fs{}\\cf{}\\highlight{}\\b{}\\i{}\\ul{}\\strike{} ",
                half_points(f.size.unwrap_or(self.points)),
                color(f.color),
                color(f.highlight),
                u8::from(f.bold.unwrap_or(false)),
                u8::from(f.italic.unwrap_or(false)),
                u8::from(f.underline.unwrap_or(false)),
                u8::from(f.strikeout.unwrap_or(false))
            ));
            literal(text, &mut out);
            out.push('}');
        }
        out.push('}');
        out
    }
}

fn half_points(points: f32) -> u32 {
    if points.is_finite() && points > 0.0 {
        (points.clamp(0.5, 1638.0) * 2.0).round() as u32
    } else {
        20
    }
}

/// RTF-escaped text (`\\`, braces, newlines, tabs and Unicode `\\uN?`).
pub(crate) fn escape_rtf(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    literal(text, &mut out);
    out
}

fn literal(text: &str, out: &mut String) {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    for c in text.encode_utf16() {
        match c {
            92 | 123 | 125 => {
                out.push('\\');
                out.push(c as u8 as char);
            }
            10 => out.push_str("\\par\n"),
            9 => out.push_str("\\tab "),
            32..=126 => out.push(c as u8 as char),
            _ => out.push_str(&format!("\\u{}?", c as i16)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mapped_styles_use_unicode_positions_and_merge_adjacent_runs() {
        let mut text = StyledText::new("Consolas", 10.0);
        let mut visited = Vec::new();
        text.push_mapped("á😀xy", |index, ch| {
            visited.push((index, ch));
            TextFormat::new().bold(index < 2)
        });
        assert_eq!(visited, vec![(0, 'á'), (1, '😀'), (2, 'x'), (3, 'y')]);
        assert_eq!(text.runs.len(), 2);
        assert_eq!(text.runs[0].0, "á😀");
        assert_eq!(text.runs[1].0, "xy");
        text.push_mapped("", |_, _| panic!("texto vazio não chama o callback"));
        assert_eq!(text.runs.len(), 2);
    }
    #[test]
    fn escapes_unicode_and_isolates_styles() {
        let mut d = StyledText::new("Consolas", 10.0);
        d.push("{\\😀}\r\n\t", TextFormat::new().bold(true).highlight(Color::RED));
        d.push("normal", TextFormat::new());
        let r = d.to_rtf();
        assert!(r.contains("\\{\\\\\\u-10179?\\u-8704?\\}"));
        assert!(r.contains("\\par\n\\tab "));
        assert!(r.contains("\\cf0\\highlight0\\b0\\i0\\ul0\\strike0 normal"));
        assert_eq!(r.matches("\\red220").count(), 1);
    }
}
