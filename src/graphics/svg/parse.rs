//! SVG values: numbers, lengths, colors, transforms, path data and simple stylesheets.

use crate::graphics::vector::{arc_segments, Figure, Matrix, Segment};
use crate::utilities::Color;

/// Space/comma-separated number reader for `points`, `viewBox`, `d`, etc.
pub(crate) struct Numbers<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Numbers<'a> {
    pub fn new(text: &'a str) -> Numbers<'a> {
        Numbers { bytes: text.as_bytes(), at: 0 }
    }

    fn skip_separators(&mut self) {
        while self.at < self.bytes.len() && (self.bytes[self.at].is_ascii_whitespace() || self.bytes[self.at] == b',') {
            self.at += 1;
        }
    }

    /// Peeks next significant character.
    pub fn peek(&mut self) -> Option<u8> {
        self.skip_separators();
        self.bytes.get(self.at).copied()
    }

    /// Consumes a character.
    pub fn bump(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.at += 1;
        Some(c)
    }

    /// Reads a number (`-1.5e3`, `.5`, `+2`).
    pub fn number(&mut self) -> Option<f64> {
        self.skip_separators();
        let start = self.at;
        let b = self.bytes;
        let mut i = self.at;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        let mut digits = false;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
            digits = true;
        }
        if i < b.len() && b[i] == b'.' {
            i += 1;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
                digits = true;
            }
        }
        if !digits {
            return None;
        }
        if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
            let mut j = i + 1;
            if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
                j += 1;
            }
            if j < b.len() && b[j].is_ascii_digit() {
                while j < b.len() && b[j].is_ascii_digit() {
                    j += 1;
                }
                i = j;
            }
        }
        self.at = i;
        std::str::from_utf8(&b[start..i]).ok()?.parse().ok()
    }

    /// Reads arc flag `0`/`1`, potentially adjacent to the next number.
    pub fn flag(&mut self) -> Option<bool> {
        match self.bump()? {
            b'0' => Some(false),
            b'1' => Some(true),
            _ => None,
        }
    }

    /// All remaining numbers.
    pub fn all(mut self) -> Vec<f64> {
        std::iter::from_fn(|| self.number()).collect()
    }
}

/// Length in user units (`12`, `12px`, `9pt`, `1in`, `2mm`, `1em`); `%` is relative to `reference`.
pub(crate) fn length(text: &str, reference: f64) -> Option<f64> {
    let text = text.trim();
    let split = text.find(|c: char| c.is_ascii_alphabetic() || c == '%').unwrap_or(text.len());
    let (number, unit) = text.split_at(split);
    let value: f64 = number.trim().parse().ok()?;
    let factor = match unit.trim() {
        "" | "px" => 1.0,
        "pt" => 96.0 / 72.0,
        "pc" => 16.0,
        "in" => 96.0,
        "cm" => 96.0 / 2.54,
        "mm" => 96.0 / 25.4,
        "em" => 16.0,
        "ex" => 8.0,
        "%" => reference / 100.0,
        _ => return None,
    };
    Some(value * factor)
}

/// Number/percentage (`0.5`, `50%`), e.g. `offset` and gradient coordinates.
pub(crate) fn fraction(text: &str) -> Option<f64> {
    let text = text.trim();
    match text.strip_suffix('%') {
        Some(percent) => percent.trim().parse::<f64>().ok().map(|p| p / 100.0),
        None => text.parse().ok(),
    }
}

const NAMED: &[(&str, u32)] = &[
    ("black", 0x000000),
    ("white", 0xFFFFFF),
    ("red", 0xFF0000),
    ("green", 0x008000),
    ("blue", 0x0000FF),
    ("yellow", 0xFFFF00),
    ("cyan", 0x00FFFF),
    ("aqua", 0x00FFFF),
    ("magenta", 0xFF00FF),
    ("fuchsia", 0xFF00FF),
    ("gray", 0x808080),
    ("grey", 0x808080),
    ("silver", 0xC0C0C0),
    ("maroon", 0x800000),
    ("olive", 0x808000),
    ("lime", 0x00FF00),
    ("teal", 0x008080),
    ("navy", 0x000080),
    ("purple", 0x800080),
    ("orange", 0xFFA500),
    ("pink", 0xFFC0CB),
    ("brown", 0xA52A2A),
    ("gold", 0xFFD700),
    ("indigo", 0x4B0082),
    ("violet", 0xEE82EE),
    ("darkgray", 0xA9A9A9),
    ("darkgrey", 0xA9A9A9),
    ("lightgray", 0xD3D3D3),
    ("lightgrey", 0xD3D3D3),
    ("dimgray", 0x696969),
    ("darkred", 0x8B0000),
    ("darkgreen", 0x006400),
    ("darkblue", 0x00008B),
    ("lightblue", 0xADD8E6),
    ("skyblue", 0x87CEEB),
    ("steelblue", 0x4682B4),
    ("royalblue", 0x4169E1),
    ("dodgerblue", 0x1E90FF),
    ("tomato", 0xFF6347),
    ("crimson", 0xDC143C),
    ("coral", 0xFF7F50),
    ("salmon", 0xFA8072),
    ("khaki", 0xF0E68C),
    ("beige", 0xF5F5DC),
    ("ivory", 0xFFFFF0),
    ("tan", 0xD2B48C),
    ("chocolate", 0xD2691E),
    ("orchid", 0xDA70D6),
    ("plum", 0xDDA0DD),
    ("turquoise", 0x40E0D0),
    ("limegreen", 0x32CD32),
    ("forestgreen", 0x228B22),
    ("seagreen", 0x2E8B57),
    ("whitesmoke", 0xF5F5F5),
    ("gainsboro", 0xDCDCDC),
    ("slategray", 0x708090),
    ("darkorange", 0xFF8C00),
    ("firebrick", 0xB22222),
];

/// CSS color: `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb()`, `rgba()`, common names. `None` if unknown.
pub(crate) fn color(text: &str) -> Option<Color> {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix('#') {
        let digit = |i: usize| u8::from_str_radix(hex.get(i..i + 1)?, 16).ok();
        let pair = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
        return match hex.len() {
            3 | 4 => {
                let alpha = if hex.len() == 4 { digit(3)? * 17 } else { 255 };
                Some(Color { r: digit(0)? * 17, g: digit(1)? * 17, b: digit(2)? * 17, a: alpha })
            }
            6 | 8 => {
                let alpha = if hex.len() == 8 { pair(6)? } else { 255 };
                Some(Color { r: pair(0)?, g: pair(2)?, b: pair(4)?, a: alpha })
            }
            _ => None,
        };
    }
    let lower = text.to_ascii_lowercase();
    if let Some(inner) = lower.strip_prefix("rgba(").or_else(|| lower.strip_prefix("rgb(")) {
        let inner = inner.strip_suffix(')')?;
        let parts: Vec<&str> = inner.split([',', ' ', '/']).filter(|p| !p.is_empty()).collect();
        if parts.len() < 3 {
            return None;
        }
        let channel = |p: &str| -> Option<u8> {
            Some(match p.strip_suffix('%') {
                Some(percent) => (percent.parse::<f64>().ok()? * 2.55).round().clamp(0.0, 255.0) as u8,
                None => p.parse::<f64>().ok()?.round().clamp(0.0, 255.0) as u8,
            })
        };
        let alpha = match parts.get(3) {
            Some(a) => (fraction(a)? * 255.0).round().clamp(0.0, 255.0) as u8,
            None => 255,
        };
        return Some(Color { r: channel(parts[0])?, g: channel(parts[1])?, b: channel(parts[2])?, a: alpha });
    }
    if lower == "transparent" {
        return Some(Color { r: 0, g: 0, b: 0, a: 0 });
    }
    NAMED.iter().find(|(name, _)| *name == lower).map(|&(_, rgb)| Color {
        r: (rgb >> 16) as u8,
        g: (rgb >> 8) as u8,
        b: rgb as u8,
        a: 255,
    })
}

/// Transform list (`matrix`, `translate`, `scale`, `rotate`, `skewX`, `skewY`).
pub(crate) fn transform(text: &str) -> Matrix {
    let mut result = Matrix::IDENTITY;
    let mut rest = text;
    while let Some(open) = rest.find('(') {
        let name = rest[..open].trim_matches(|c: char| c.is_whitespace() || c == ',');
        let Some(close) = rest[open..].find(')') else { break };
        let args = Numbers::new(&rest[open + 1..open + close]).all();
        rest = &rest[open + close + 1..];
        let arg = |i: usize, default: f64| args.get(i).copied().unwrap_or(default);
        let m = match name {
            "matrix" if args.len() == 6 => Matrix { m: [args[0], args[1], args[2], args[3], args[4], args[5]] },
            "translate" => Matrix::translate(arg(0, 0.0), arg(1, 0.0)),
            "scale" => Matrix::scale(arg(0, 1.0), arg(1, arg(0, 1.0))),
            "rotate" => {
                let (cx, cy) = (arg(1, 0.0), arg(2, 0.0));
                Matrix::translate(cx, cy).multiply(&Matrix::rotate(arg(0, 0.0))).multiply(&Matrix::translate(-cx, -cy))
            }
            "skewX" => Matrix { m: [1.0, 0.0, arg(0, 0.0).to_radians().tan(), 1.0, 0.0, 0.0] },
            "skewY" => Matrix { m: [1.0, arg(0, 0.0).to_radians().tan(), 0.0, 1.0, 0.0, 0.0] },
            _ => Matrix::IDENTITY,
        };
        result = result.multiply(&m);
    }
    result
}

/// Path `d`: all absolute/relative commands; arcs converted to curves. Errors stop parsing while retaining parsed
/// content, like browsers.
pub(crate) fn path(data: &str) -> Vec<Figure> {
    let mut figures: Vec<Figure> = Vec::new();
    let mut numbers = Numbers::new(data);
    let mut command = 0u8;
    let mut current = (0.0f64, 0.0f64);
    let mut start = (0.0f64, 0.0f64);
    // Último ponto de controle (para S/s e T/t) e de que tipo era.
    let mut last_cubic: Option<(f64, f64)> = None;
    let mut last_quad: Option<(f64, f64)> = None;
    while let Some(next) = numbers.peek() {
        if next.is_ascii_alphabetic() {
            command = next;
            numbers.bump();
            if command == b'Z' || command == b'z' {
                if let Some(figure) = figures.last_mut() {
                    figure.closed = true;
                }
                current = start;
                last_cubic = None;
                last_quad = None;
                continue;
            }
        } else if command == 0 {
            break;
        }
        let relative = command.is_ascii_lowercase();
        let base = if relative { current } else { (0.0, 0.0) };
        let point = |n: &mut Numbers| -> Option<(f64, f64)> { Some((base.0 + n.number()?, base.1 + n.number()?)) };
        let ensure_figure = |figures: &mut Vec<Figure>, at: (f64, f64)| {
            if figures.last().is_none_or_closed() {
                figures.push(Figure::new(at));
            }
        };
        let parsed: Option<()> = (|| {
            match command.to_ascii_uppercase() {
                b'M' => {
                    let p = point(&mut numbers)?;
                    figures.push(Figure::new(p));
                    current = p;
                    start = p;
                    // Pares seguintes ao M são retas.
                    command = if relative { b'l' } else { b'L' };
                    last_cubic = None;
                    last_quad = None;
                }
                b'L' => {
                    let p = point(&mut numbers)?;
                    ensure_figure(&mut figures, current);
                    figures.last_mut()?.segments.push(Segment::Line(p));
                    current = p;
                    last_cubic = None;
                    last_quad = None;
                }
                b'H' => {
                    let x = numbers.number()? + if relative { current.0 } else { 0.0 };
                    ensure_figure(&mut figures, current);
                    current = (x, current.1);
                    figures.last_mut()?.segments.push(Segment::Line(current));
                    last_cubic = None;
                    last_quad = None;
                }
                b'V' => {
                    let y = numbers.number()? + if relative { current.1 } else { 0.0 };
                    ensure_figure(&mut figures, current);
                    current = (current.0, y);
                    figures.last_mut()?.segments.push(Segment::Line(current));
                    last_cubic = None;
                    last_quad = None;
                }
                b'C' | b'S' => {
                    let c1 = if command.eq_ignore_ascii_case(&b'C') {
                        point(&mut numbers)?
                    } else {
                        last_cubic.map_or(current, |c| (2.0 * current.0 - c.0, 2.0 * current.1 - c.1))
                    };
                    let c2 = point(&mut numbers)?;
                    let p = point(&mut numbers)?;
                    ensure_figure(&mut figures, current);
                    figures.last_mut()?.segments.push(Segment::Cubic(c1, c2, p));
                    current = p;
                    last_cubic = Some(c2);
                    last_quad = None;
                }
                b'Q' | b'T' => {
                    let q = if command.eq_ignore_ascii_case(&b'Q') {
                        point(&mut numbers)?
                    } else {
                        last_quad.map_or(current, |c| (2.0 * current.0 - c.0, 2.0 * current.1 - c.1))
                    };
                    let p = point(&mut numbers)?;
                    ensure_figure(&mut figures, current);
                    let c1 = (current.0 + 2.0 / 3.0 * (q.0 - current.0), current.1 + 2.0 / 3.0 * (q.1 - current.1));
                    let c2 = (p.0 + 2.0 / 3.0 * (q.0 - p.0), p.1 + 2.0 / 3.0 * (q.1 - p.1));
                    figures.last_mut()?.segments.push(Segment::Cubic(c1, c2, p));
                    current = p;
                    last_quad = Some(q);
                    last_cubic = None;
                }
                b'A' => {
                    let (rx, ry, rotation) = (numbers.number()?, numbers.number()?, numbers.number()?);
                    let (large, sweep) = (numbers.flag()?, numbers.flag()?);
                    let p = point(&mut numbers)?;
                    ensure_figure(&mut figures, current);
                    let figure = figures.last_mut()?;
                    arc_to(figure, current, rx, ry, rotation, large, sweep, p);
                    current = p;
                    last_cubic = None;
                    last_quad = None;
                }
                _ => return None,
            }
            Some(())
        })();
        if parsed.is_none() {
            break;
        }
    }
    figures.retain(|f| !f.segments.is_empty() || f.closed);
    figures
}

trait LastFigure {
    fn is_none_or_closed(&self) -> bool;
}

impl LastFigure for Option<&Figure> {
    fn is_none_or_closed(&self) -> bool {
        self.map_or(true, |f| f.closed)
    }
}

/// SVG endpoint elliptical arc converted to center form.
#[allow(clippy::too_many_arguments)]
fn arc_to(
    figure: &mut Figure,
    from: (f64, f64),
    rx: f64,
    ry: f64,
    rotation: f64,
    large: bool,
    sweep: bool,
    to: (f64, f64),
) {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx < 1e-9 || ry < 1e-9 || (from.0 - to.0).abs() + (from.1 - to.1).abs() < 1e-12 {
        if (from.0 - to.0).abs() + (from.1 - to.1).abs() >= 1e-12 {
            figure.segments.push(Segment::Line(to));
        }
        return;
    }
    let phi = rotation.to_radians();
    let (sin, cos) = phi.sin_cos();
    let (dx, dy) = ((from.0 - to.0) / 2.0, (from.1 - to.1) / 2.0);
    let (x1, y1) = (cos * dx + sin * dy, -sin * dx + cos * dy);
    let lambda = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }
    let numerator = (rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1).max(0.0);
    let denominator = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let mut factor = (numerator / denominator).sqrt();
    if large == sweep {
        factor = -factor;
    }
    let (cx1, cy1) = (factor * rx * y1 / ry, -factor * ry * x1 / rx);
    let cx = cos * cx1 - sin * cy1 + (from.0 + to.0) / 2.0;
    let cy = sin * cx1 + cos * cy1 + (from.1 + to.1) / 2.0;
    let angle = |ux: f64, uy: f64, vx: f64, vy: f64| (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
    let start = angle(1.0, 0.0, (x1 - cx1) / rx, (y1 - cy1) / ry);
    let mut delta = angle((x1 - cx1) / rx, (y1 - cy1) / ry, (-x1 - cx1) / rx, (-y1 - cy1) / ry);
    if !sweep && delta > 0.0 {
        delta -= std::f64::consts::TAU;
    } else if sweep && delta < 0.0 {
        delta += std::f64::consts::TAU;
    }
    arc_segments(figure, cx, cy, rx, ry, phi, start, delta);
    // Garante o ponto final exato (evita frestas por arredondamento).
    if let Some(Segment::Cubic(_, _, end)) = figure.segments.last_mut() {
        *end = to;
    }
}

/// Simple CSS rule: selectors (`tag`, `.class`, `#id`, `*`, `tag.class`) and declarations.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CssRule {
    pub selector: Selector,
    pub declarations: Vec<(String, String)>,
    pub order: usize,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct Selector {
    pub tag: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
}

impl Selector {
    fn parse(text: &str) -> Option<Selector> {
        let text = text.trim();
        if text.is_empty() || text.contains([' ', '>', '+', '~', '[', ':']) {
            return None;
        }
        let mut selector = Selector::default();
        let mut rest = text;
        let first = rest.find(['.', '#']).unwrap_or(rest.len());
        if first > 0 && &rest[..first] != "*" {
            selector.tag = Some(rest[..first].to_string());
        }
        rest = &rest[first..];
        while !rest.is_empty() {
            let kind = rest.as_bytes()[0];
            let end = rest[1..].find(['.', '#']).map_or(rest.len(), |e| e + 1);
            let name = rest[1..end].to_string();
            if kind == b'.' {
                selector.classes.push(name);
            } else {
                selector.id = Some(name);
            }
            rest = &rest[end..];
        }
        Some(selector)
    }

    pub fn specificity(&self) -> (usize, usize, usize) {
        (self.id.is_some() as usize, self.classes.len(), self.tag.is_some() as usize)
    }

    pub fn matches(&self, tag: &str, id: Option<&str>, classes: &[&str]) -> bool {
        self.tag.as_deref().is_none_or_eq(tag)
            && self.id.as_deref().is_none_or_eq_opt(id)
            && self.classes.iter().all(|c| classes.contains(&c.as_str()))
    }
}

trait OptionalMatch {
    fn is_none_or_eq(&self, value: &str) -> bool;
    fn is_none_or_eq_opt(&self, value: Option<&str>) -> bool;
}

impl OptionalMatch for Option<&str> {
    fn is_none_or_eq(&self, value: &str) -> bool {
        self.map_or(true, |s| s == value)
    }
    fn is_none_or_eq_opt(&self, value: Option<&str>) -> bool {
        self.map_or(true, |s| Some(s) == value)
    }
}

/// `property: value; ...` declarations, as in `style` attributes.
pub(crate) fn declarations(text: &str) -> Vec<(String, String)> {
    text.split(';')
        .filter_map(|d| {
            let (key, value) = d.split_once(':')?;
            let value = value.trim().trim_end_matches("!important").trim();
            Some((key.trim().to_ascii_lowercase(), value.to_string()))
        })
        .filter(|(k, v)| !k.is_empty() && !v.is_empty())
        .collect()
}

/// Stylesheet with simple selectors; unsupported rules ignored.
pub(crate) fn stylesheet(text: &str, first_order: usize) -> Vec<CssRule> {
    let mut text = text.to_string();
    while let Some(start) = text.find("/*") {
        let end = text[start..].find("*/").map_or(text.len(), |e| start + e + 2);
        text.replace_range(start..end, "");
    }
    let mut rules = Vec::new();
    let mut rest = text.as_str();
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}') else { break };
        let selectors = &rest[..open];
        let body = declarations(&rest[open + 1..open + close]);
        if !selectors.trim_start().starts_with('@') {
            for selector in selectors.split(',').filter_map(Selector::parse) {
                rules.push(CssRule { selector, declarations: body.clone(), order: first_order + rules.len() });
            }
        }
        rest = &rest[open + close + 1..];
    }
    rules
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_lengths_and_colors() {
        assert_eq!(Numbers::new("1,2 -3.5e1.5-.5+4").all(), vec![1.0, 2.0, -35.0, 0.5, -0.5, 4.0]);
        assert_eq!(length("12pt", 0.0), Some(16.0));
        assert_eq!(length("50%", 200.0), Some(100.0));
        assert_eq!(length("1in", 0.0), Some(96.0));
        assert_eq!(length("abc", 0.0), None);
        assert_eq!(fraction("25%"), Some(0.25));
        assert_eq!(color("#f80"), Some(Color::rgb(255, 136, 0)));
        assert_eq!(color("#12345678"), Some(Color { r: 0x12, g: 0x34, b: 0x56, a: 0x78 }));
        assert_eq!(color("rgb(10, 20%, 30)"), Some(Color::rgb(10, 51, 30)));
        assert_eq!(color("rgba(0,0,0,0.5)").map(|c| c.a), Some(128));
        assert_eq!(color("SteelBlue"), Some(Color::rgb(0x46, 0x82, 0xB4)));
        assert_eq!(color("nada"), None);
    }

    #[test]
    fn transforms() {
        let m = transform("translate(10 20) scale(2)");
        assert_eq!(m.apply((1.0, 1.0)), (12.0, 22.0));
        let r = transform("rotate(90, 10, 10)");
        let p = r.apply((20.0, 10.0));
        assert!((p.0 - 10.0).abs() < 1e-9 && (p.1 - 20.0).abs() < 1e-9);
        let m = transform("matrix(1 0 0 1 5 6)");
        assert_eq!(m.apply((0.0, 0.0)), (5.0, 6.0));
        assert_eq!(transform("invalido"), Matrix::IDENTITY);
    }

    #[test]
    fn path_commands() {
        let figures = path("M10 10 h10 v10 H10 z m5 5 l1 1 2 2");
        assert_eq!(figures.len(), 2);
        assert!(figures[0].closed);
        assert_eq!(figures[0].segments.len(), 3);
        assert_eq!(figures[0].end(), (10.0, 20.0));
        assert_eq!(figures[1].start, (15.0, 15.0), "m relativo ao início da figura fechada");
        assert_eq!(figures[1].end(), (18.0, 18.0), "pares implícitos depois do m viram l");
        let curves = path("M0 0C0 10 10 10 10 0S20 -10 20 0Q25 5 30 0T40 0");
        assert_eq!(curves[0].segments.len(), 4);
        match curves[0].segments[1] {
            Segment::Cubic(c1, _, end) => {
                assert_eq!(c1, (10.0, -10.0), "S reflete o controle anterior");
                assert_eq!(end, (20.0, 0.0));
            }
            _ => panic!("esperava curva"),
        }
        let arc = path("M0 0 A10 10 0 0 1 20 0");
        assert_eq!(arc[0].end(), (20.0, 0.0));
        assert_eq!(arc[0].segments.len(), 2, "meia volta = duas curvas de 90°");
        let flags = path("M0 0a5 5 0 1020 0");
        assert_eq!(flags[0].end(), (20.0, 0.0), "bandeiras coladas");
        let broken = path("M0 0 L10 10 L20 x L30 30");
        assert_eq!(broken[0].segments.len(), 1, "para no primeiro erro, mantendo o que leu");
        assert!(path("").is_empty());
    }

    #[test]
    fn css_rules() {
        let rules = stylesheet(
            "/* c */ .a, rect { fill: red; stroke : blue } #x.b{opacity:.5} @media x { } g > rect {fill:none}",
            0,
        );
        assert_eq!(rules.len(), 3);
        assert!(rules[0].selector.matches("path", None, &["a"]));
        assert!(!rules[0].selector.matches("path", None, &["b"]));
        assert!(rules[1].selector.matches("rect", None, &[]));
        assert_eq!(rules[2].selector.specificity(), (1, 1, 0));
        assert!(rules[2].selector.matches("g", Some("x"), &["b", "c"]));
        assert_eq!(
            declarations("fill:red; stroke-width : 2 !important;;"),
            vec![("fill".to_string(), "red".to_string()), ("stroke-width".to_string(), "2".to_string())]
        );
    }
}
