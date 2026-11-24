//! Own dependency-free SVG renderer: SVG 1.1 subset for icons/illustrations, antialiased with GDI+.
//!
//! Supported: `svg` (including nested), `g`, `a`, `use`/`symbol`, `defs`, `path` (all commands including arcs), `rect`
//! (rounded corners), `circle`, `ellipse`, `line`, `polyline`, `polygon`; `linearGradient`/`radialGradient` (stops,
//! `href`, `gradientUnits`, `gradientTransform`, `spreadMethod`); `transform`; fill/stroke (color, `url(#…)`,
//! `currentColor`, opacity, `fill-rule`, width, join, cap, miter limit, dashes); `opacity`, `display`, `visibility`;
//! `style` attributes and `<style>` with simple selectors (`tag`, `.class`, `#id`); `viewBox` and
//! `preserveAspectRatio`.
//!
//! Unsupported/ignored: text, embedded images, `clipPath`, `mask`, `pattern`, filters, markers and animations. Group
//! `opacity` applies per element, making overlaps darker than in browsers.

mod parse;
mod xml;

use crate::core::{GuiError, GuiResult};
use crate::graphics::vector::{
    bounds, Figure, LineCap, LineJoin, Matrix, Paint, Segment, Spread, Stroke, VectorPainter,
};
use crate::graphics::{PlusGraphics, PlusImage};
use crate::utilities::{Color, Size};
use parse::{color, fraction, length, CssRule, Numbers};
use std::collections::HashMap;
use xml::Element;

/// Resolved drawing operation (user space plus transform).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DrawOp {
    Fill { figures: Vec<Figure>, even_odd: bool, paint: Paint, opacity: f64, transform: Matrix },
    Stroke { figures: Vec<Figure>, stroke: Stroke, paint: Paint, opacity: f64, transform: Matrix },
}

/// Parsed SVG ready to draw at any size.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo() -> GuiResult<()> {
/// let svg = SvgImage::parse(r#"<svg viewBox="0 0 10 10"><circle cx="5" cy="5" r="4" fill="teal"/></svg>"#)?;
/// let image = svg.render(64, 64)?; // PlusImage with transparency
/// image.save_png("circle.png")?;
/// # Ok(()) }
/// ```
#[derive(Clone, Debug)]
pub struct SvgImage {
    root: Element,
    ids: HashMap<String, Element>,
    rules: Vec<CssRule>,
    width: f64,
    height: f64,
    view_box: Option<(f64, f64, f64, f64)>,
    current_color: Color,
}

/// Inherited drawing properties.
#[derive(Clone, Debug)]
struct Style {
    fill: Option<String>,
    fill_opacity: f64,
    even_odd: bool,
    stroke_paint: Option<String>,
    stroke_opacity: f64,
    stroke: Stroke,
    color: Color,
    visible: bool,
    opacity: f64,
}

impl SvgImage {
    /// Parses an SVG document.
    pub fn parse(source: &str) -> GuiResult<SvgImage> {
        let root = xml::parse(source).map_err(|e| GuiError::InvalidArgument(format!("invalid SVG: {e}")))?;
        if root.local_name() != "svg" {
            return Err(GuiError::InvalidArgument(format!("invalid SVG: root <{}>", root.name)));
        }
        let mut ids = HashMap::new();
        let mut rules = Vec::new();
        collect(&root, &mut ids, &mut rules);
        let view_box = root.attribute("viewBox").and_then(|v| {
            let n = Numbers::new(v).all();
            (n.len() == 4 && n[2] > 0.0 && n[3] > 0.0).then(|| (n[0], n[1], n[2], n[3]))
        });
        let (default_w, default_h) = view_box.map_or((300.0, 150.0), |v| (v.2, v.3));
        let dimension = |name: &str, default: f64| {
            root.attribute(name)
                .and_then(|v| if v.trim().ends_with('%') { None } else { length(v, default) })
                .filter(|v| *v > 0.0)
                .unwrap_or(default)
        };
        let (mut width, mut height) = (dimension("width", default_w), dimension("height", default_h));
        // Só um lado informado: o outro segue a proporção do viewBox.
        if let Some(v) = view_box {
            match (root.attribute("width").is_some(), root.attribute("height").is_some()) {
                (true, false) => height = width * v.3 / v.2,
                (false, true) => width = height * v.2 / v.3,
                _ => {}
            }
        }
        Ok(SvgImage { root, ids, rules, width, height, view_box, current_color: Color::BLACK })
    }

    /// Parses a UTF-8 SVG document.
    pub fn from_bytes(bytes: &[u8]) -> GuiResult<SvgImage> {
        let text = std::str::from_utf8(bytes).map_err(|_| GuiError::InvalidArgument("SVG is not UTF-8".into()))?;
        SvgImage::parse(text)
    }

    /// Reads an `.svg` file.
    pub fn from_file(path: impl AsRef<std::path::Path>) -> GuiResult<SvgImage> {
        let bytes = std::fs::read(path.as_ref())
            .map_err(|e| GuiError::InvalidArgument(format!("could not read the SVG: {e}")))?;
        SvgImage::from_bytes(&bytes)
    }

    /// Rounded natural size (document `width`/`height` or `viewBox`).
    pub fn size(&self) -> Size {
        Size::new(self.width.round().max(1.0) as i32, self.height.round().max(1.0) as i32)
    }

    /// `viewBox` (`x`, `y`, width, height), if present.
    pub fn view_box(&self) -> Option<(f64, f64, f64, f64)> {
        self.view_box
    }

    /// `currentColor` value (default black), e.g. theme text color.
    pub fn set_current_color(&mut self, color: Color) {
        self.current_color = color;
    }

    /// Same image with another `currentColor`.
    pub fn with_current_color(mut self, color: Color) -> SvgImage {
        self.current_color = color;
        self
    }

    /// Renders a new transparent `width` × `height` pixel image.
    pub fn render(&self, width: i32, height: i32) -> GuiResult<PlusImage> {
        if width <= 0 || height <= 0 {
            return Err(GuiError::InvalidArgument("invalid size for the SVG".into()));
        }
        let image = PlusImage::from_pixels(width, height, &vec![0u8; width as usize * height as usize * 4])?;
        {
            let graphics = PlusGraphics::for_image(&image)?;
            self.draw(&graphics, 0.0, 0.0, width as f64, height as f64);
        }
        Ok(image)
    }

    /// Draws in the given GDI+ context rectangle using context coordinates.
    pub fn draw(&self, graphics: &PlusGraphics, x: f64, y: f64, width: f64, height: f64) {
        let painter = VectorPainter { graphics };
        for op in self.operations(x, y, width, height) {
            match op {
                DrawOp::Fill { figures, even_odd, paint, opacity, transform } => {
                    painter.fill_in(&transform, &figures, even_odd, &paint, opacity)
                }
                DrawOp::Stroke { figures, stroke, paint, opacity, transform } => {
                    painter.stroke_in(&transform, &figures, &stroke, &paint, opacity)
                }
            }
        }
    }

    /// Drawing operations for the destination rectangle.
    pub(crate) fn operations(&self, x: f64, y: f64, width: f64, height: f64) -> Vec<DrawOp> {
        let viewport = Matrix::translate(x, y).multiply(&viewbox_transform(
            self.view_box.unwrap_or((0.0, 0.0, self.width, self.height)),
            width,
            height,
            self.root.attribute("preserveAspectRatio").unwrap_or(""),
        ));
        let style = Style {
            fill: Some("black".into()),
            fill_opacity: 1.0,
            even_odd: false,
            stroke_paint: None,
            stroke_opacity: 1.0,
            stroke: Stroke::default(),
            color: self.current_color,
            visible: true,
            opacity: 1.0,
        };
        let mut ops = Vec::new();
        let mut renderer = Renderer { svg: self, ops: &mut ops, depth: 0 };
        renderer.children(&self.root, &style, &viewport);
        ops
    }
}

fn collect(element: &Element, ids: &mut HashMap<String, Element>, rules: &mut Vec<CssRule>) {
    if let Some(id) = element.attribute("id") {
        ids.entry(id.to_string()).or_insert_with(|| element.clone());
    }
    if element.local_name() == "style" {
        let first = rules.len();
        rules.extend(parse::stylesheet(&element.text, first));
    }
    for child in &element.children {
        collect(child, ids, rules);
    }
}

/// `viewBox` transform to `width` × `height` with `preserveAspectRatio`.
pub(crate) fn viewbox_transform(view_box: (f64, f64, f64, f64), width: f64, height: f64, aspect: &str) -> Matrix {
    let (vx, vy, vw, vh) = view_box;
    if vw <= 0.0 || vh <= 0.0 {
        return Matrix::IDENTITY;
    }
    let (sx, sy) = (width / vw, height / vh);
    let mut parts = aspect.split_whitespace();
    let align = parts.next().unwrap_or("xMidYMid");
    if align == "none" {
        return Matrix::scale(sx, sy).multiply(&Matrix::translate(-vx, -vy));
    }
    let slice = parts.next() == Some("slice");
    let s = if slice { sx.max(sy) } else { sx.min(sy) };
    let (free_x, free_y) = (width - vw * s, height - vh * s);
    let offset = |min: &str, max: &str, free: f64| {
        if align.contains(min) {
            0.0
        } else if align.contains(max) {
            free
        } else {
            free / 2.0
        }
    };
    Matrix::translate(offset("xMin", "xMax", free_x), offset("YMin", "YMax", free_y))
        .multiply(&Matrix::scale(s, s))
        .multiply(&Matrix::translate(-vx, -vy))
}

const PRESENTATION: &[&str] = &[
    "fill",
    "fill-opacity",
    "fill-rule",
    "stroke",
    "stroke-opacity",
    "stroke-width",
    "stroke-linejoin",
    "stroke-linecap",
    "stroke-miterlimit",
    "stroke-dasharray",
    "opacity",
    "display",
    "visibility",
    "color",
    "stop-color",
    "stop-opacity",
];

struct Renderer<'a> {
    svg: &'a SvgImage,
    ops: &'a mut Vec<DrawOp>,
    depth: usize,
}

impl Renderer<'_> {
    /// Element properties in increasing priority: presentation attributes, CSS by specificity, then `style` attribute.
    fn properties(&self, element: &Element) -> Vec<(String, String)> {
        let mut props: Vec<(String, String)> = element
            .attributes
            .iter()
            .filter(|(k, _)| PRESENTATION.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.trim().to_string()))
            .collect();
        if !self.svg.rules.is_empty() {
            let classes: Vec<&str> = element.attribute("class").map_or(Vec::new(), |c| c.split_whitespace().collect());
            let mut matched: Vec<&CssRule> = self
                .svg
                .rules
                .iter()
                .filter(|r| r.selector.matches(element.local_name(), element.attribute("id"), &classes))
                .collect();
            matched.sort_by_key(|r| (r.selector.specificity(), r.order));
            for rule in matched {
                props.extend(rule.declarations.iter().cloned());
            }
        }
        if let Some(style) = element.attribute("style") {
            props.extend(parse::declarations(style));
        }
        props
    }

    fn style_for(&self, element: &Element, parent: &Style) -> Option<Style> {
        let props = self.properties(element);
        let get = |name: &str| props.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.as_str());
        if get("display") == Some("none") {
            return None;
        }
        let mut style = parent.clone();
        let number = |name: &str| get(name).and_then(|v| v.trim().parse::<f64>().ok());
        if let Some(c) = get("color").and_then(color) {
            style.color = c;
        }
        if let Some(fill) = get("fill") {
            style.fill = if fill == "none" { None } else { Some(fill.to_string()) };
        }
        if let Some(stroke) = get("stroke") {
            style.stroke_paint = if stroke == "none" { None } else { Some(stroke.to_string()) };
        }
        if let Some(v) = number("fill-opacity") {
            style.fill_opacity = v.clamp(0.0, 1.0);
        }
        if let Some(v) = number("stroke-opacity") {
            style.stroke_opacity = v.clamp(0.0, 1.0);
        }
        if let Some(v) = get("fill-rule") {
            style.even_odd = v == "evenodd";
        }
        if let Some(v) = get("stroke-width").and_then(|v| length(v, 100.0)) {
            style.stroke.width = v.max(0.0);
        }
        if let Some(v) = get("stroke-linejoin") {
            style.stroke.join = match v {
                "round" => LineJoin::Round,
                "bevel" => LineJoin::Bevel,
                _ => LineJoin::Miter,
            };
        }
        if let Some(v) = get("stroke-linecap") {
            style.stroke.cap = match v {
                "round" => LineCap::Round,
                "square" => LineCap::Square,
                _ => LineCap::Butt,
            };
        }
        if let Some(v) = number("stroke-miterlimit") {
            style.stroke.miter_limit = v;
        }
        if let Some(v) = get("stroke-dasharray") {
            style.stroke.dashes = if v == "none" { Vec::new() } else { Numbers::new(v).all() };
        }
        if let Some(v) = get("visibility") {
            style.visible = v == "visible";
        }
        if let Some(v) = number("opacity") {
            style.opacity *= v.clamp(0.0, 1.0);
        }
        Some(style)
    }

    fn children(&mut self, element: &Element, style: &Style, transform: &Matrix) {
        for child in &element.children {
            self.element(child, style, transform);
        }
    }

    fn element(&mut self, element: &Element, parent: &Style, transform: &Matrix) {
        if self.depth > 64 {
            return;
        }
        let name = element.local_name();
        if matches!(
            name,
            "defs"
                | "symbol"
                | "linearGradient"
                | "radialGradient"
                | "clipPath"
                | "mask"
                | "pattern"
                | "filter"
                | "style"
                | "title"
                | "desc"
                | "metadata"
                | "text"
                | "image"
                | "marker"
                | "script"
        ) {
            return;
        }
        let Some(style) = self.style_for(element, parent) else { return };
        let mut transform = *transform;
        if let Some(t) = element.attribute("transform") {
            transform = transform.multiply(&parse::transform(t));
        }
        let number =
            |name: &str, reference: f64| element.attribute(name).and_then(|v| length(v, reference)).unwrap_or(0.0);
        let (vw, vh) = self.svg.view_box.map_or((self.svg.width, self.svg.height), |v| (v.2, v.3));
        let diagonal = ((vw * vw + vh * vh) / 2.0).sqrt();
        let figures = match name {
            "g" | "a" | "switch" => {
                self.depth += 1;
                self.children(element, &style, &transform);
                self.depth -= 1;
                return;
            }
            "svg" => {
                let (x, y) = (number("x", vw), number("y", vh));
                let width = element.attribute("width").and_then(|v| length(v, vw)).unwrap_or(vw);
                let height = element.attribute("height").and_then(|v| length(v, vh)).unwrap_or(vh);
                let inner = element
                    .attribute("viewBox")
                    .map(|v| Numbers::new(v).all())
                    .filter(|n| n.len() == 4 && n[2] > 0.0 && n[3] > 0.0)
                    .map_or(Matrix::IDENTITY, |n| {
                        viewbox_transform(
                            (n[0], n[1], n[2], n[3]),
                            width,
                            height,
                            element.attribute("preserveAspectRatio").unwrap_or(""),
                        )
                    });
                let transform = transform.multiply(&Matrix::translate(x, y)).multiply(&inner);
                self.depth += 1;
                self.children(element, &style, &transform);
                self.depth -= 1;
                return;
            }
            "use" => {
                let Some(target) = element
                    .attribute("href")
                    .and_then(|h| h.strip_prefix('#'))
                    .and_then(|id| self.svg.ids.get(id))
                    .cloned()
                else {
                    return;
                };
                let transform = transform.multiply(&Matrix::translate(number("x", vw), number("y", vh)));
                self.depth += 1;
                if target.local_name() == "symbol" {
                    if let Some(symbol_style) = self.style_for(&target, &style) {
                        self.children(&target, &symbol_style, &transform);
                    }
                } else {
                    self.element(&target, &style, &transform);
                }
                self.depth -= 1;
                return;
            }
            "path" => parse::path(element.attribute("d").unwrap_or("")),
            "rect" => {
                let (x, y) = (number("x", vw), number("y", vh));
                let (w, h) = (number("width", vw), number("height", vh));
                if w <= 0.0 || h <= 0.0 {
                    return;
                }
                let rx = element.attribute("rx").and_then(|v| length(v, vw));
                let ry = element.attribute("ry").and_then(|v| length(v, vh));
                let (rx, ry) = match (rx, ry) {
                    (Some(rx), Some(ry)) => (rx, ry),
                    (Some(r), None) | (None, Some(r)) => (r, r),
                    (None, None) => (0.0, 0.0),
                };
                vec![rounded_rect(x, y, w, h, rx.clamp(0.0, w / 2.0), ry.clamp(0.0, h / 2.0))]
            }
            "circle" => {
                let r = number("r", diagonal);
                if r <= 0.0 {
                    return;
                }
                vec![Figure::ellipse(number("cx", vw), number("cy", vh), r, r)]
            }
            "ellipse" => {
                let (rx, ry) = (number("rx", vw), number("ry", vh));
                if rx <= 0.0 || ry <= 0.0 {
                    return;
                }
                vec![Figure::ellipse(number("cx", vw), number("cy", vh), rx, ry)]
            }
            "line" => {
                let mut figure = Figure::new((number("x1", vw), number("y1", vh)));
                figure.segments.push(Segment::Line((number("x2", vw), number("y2", vh))));
                vec![figure]
            }
            "polyline" | "polygon" => {
                let points = Numbers::new(element.attribute("points").unwrap_or("")).all();
                if points.len() < 4 {
                    return;
                }
                let mut figure = Figure::new((points[0], points[1]));
                for pair in points[2..].chunks_exact(2) {
                    figure.segments.push(Segment::Line((pair[0], pair[1])));
                }
                figure.closed = name == "polygon";
                vec![figure]
            }
            _ => return,
        };
        if figures.is_empty() || !style.visible {
            return;
        }
        let is_line = matches!(name, "line" | "polyline");
        if let (Some(fill), false) = (&style.fill, is_line) {
            if let Some(paint) = self.paint(fill, &style, &figures) {
                self.ops.push(DrawOp::Fill {
                    figures: figures.clone(),
                    even_odd: style.even_odd,
                    paint,
                    opacity: style.fill_opacity * style.opacity,
                    transform,
                });
            }
        }
        if let Some(stroke) = &style.stroke_paint {
            if style.stroke.width > 0.0 {
                if let Some(paint) = self.paint(stroke, &style, &figures) {
                    self.ops.push(DrawOp::Stroke {
                        figures,
                        stroke: style.stroke.clone(),
                        paint,
                        opacity: style.stroke_opacity * style.opacity,
                        transform,
                    });
                }
            }
        }
    }

    /// Resolves paint (`#color`, `url(#id) fallback`, `currentColor`).
    fn paint(&self, value: &str, style: &Style, figures: &[Figure]) -> Option<Paint> {
        let value = value.trim();
        if let Some(rest) = value.strip_prefix("url(") {
            let close = rest.find(')')?;
            let id = rest[..close].trim().trim_matches(['\'', '"']).trim_start_matches('#');
            let fallback = rest[close + 1..].trim();
            return match self.svg.ids.get(id).and_then(|g| self.gradient(g, style, figures)) {
                Some(paint) => Some(paint),
                None if !fallback.is_empty() && fallback != "none" => self.paint(fallback, style, figures),
                None => None,
            };
        }
        if value.eq_ignore_ascii_case("currentcolor") {
            return Some(Paint::Solid(style.color));
        }
        color(value).map(Paint::Solid)
    }

    /// `<linearGradient>`/`<radialGradient>` including `href` chain.
    fn gradient(&self, element: &Element, style: &Style, figures: &[Figure]) -> Option<Paint> {
        let kind = element.local_name();
        if kind != "linearGradient" && kind != "radialGradient" {
            return None;
        }
        let mut chain = vec![element];
        while chain.len() < 8 {
            let next = chain[chain.len() - 1]
                .attribute("href")
                .and_then(|h| h.strip_prefix('#'))
                .and_then(|id| self.svg.ids.get(id));
            match next {
                Some(next) if !chain.iter().any(|e| std::ptr::eq(*e, next)) => chain.push(next),
                _ => break,
            }
        }
        let attribute = |name: &str| chain.iter().find_map(|e| e.attribute(name));
        let stops_owner = chain.iter().find(|e| e.children.iter().any(|c| c.local_name() == "stop"))?;
        let mut stops = Vec::new();
        for stop in stops_owner.children.iter().filter(|c| c.local_name() == "stop") {
            let props = self.properties(stop);
            let get = |name: &str| props.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.clone());
            let offset = stop.attribute("offset").and_then(fraction).unwrap_or(0.0).clamp(0.0, 1.0);
            let mut c = match get("stop-color") {
                Some(v) if v.eq_ignore_ascii_case("currentcolor") => style.color,
                Some(v) => color(&v).unwrap_or(Color::BLACK),
                None => Color::BLACK,
            };
            let opacity = get("stop-opacity").and_then(|v| v.trim().parse::<f64>().ok()).unwrap_or(1.0);
            c.a = (c.a as f64 * opacity.clamp(0.0, 1.0)).round() as u8;
            stops.push((offset, c));
        }
        if stops.is_empty() {
            return None;
        }
        if stops.len() == 1 {
            return Some(Paint::Solid(stops[0].1));
        }
        let user_space = attribute("gradientUnits") == Some("userSpaceOnUse");
        let mut transform = Matrix::IDENTITY;
        let (vw, vh) = self.svg.view_box.map_or((self.svg.width, self.svg.height), |v| (v.2, v.3));
        if !user_space {
            let (x0, y0, x1, y1) = bounds(figures)?;
            if x1 - x0 <= 0.0 || y1 - y0 <= 0.0 {
                return None;
            }
            transform = Matrix { m: [x1 - x0, 0.0, 0.0, y1 - y0, x0, y0] };
        }
        if let Some(t) = attribute("gradientTransform") {
            transform = transform.multiply(&parse::transform(t));
        }
        let coordinate = |name: &str, default: f64, reference: f64| -> f64 {
            match attribute(name) {
                Some(v) if user_space => length(v, reference).unwrap_or(default * reference),
                Some(v) => fraction(v).unwrap_or(default),
                None if user_space => default * reference,
                None => default,
            }
        };
        if kind == "linearGradient" {
            let spread = match attribute("spreadMethod") {
                Some("repeat") => Spread::Repeat,
                Some("reflect") => Spread::Reflect,
                _ => Spread::Pad,
            };
            Some(Paint::Linear {
                start: (coordinate("x1", 0.0, vw), coordinate("y1", 0.0, vh)),
                end: (coordinate("x2", 1.0, vw), coordinate("y2", 0.0, vh)),
                stops,
                transform,
                spread,
            })
        } else {
            let diagonal = ((vw * vw + vh * vh) / 2.0).sqrt();
            let center = (coordinate("cx", 0.5, vw), coordinate("cy", 0.5, vh));
            let radius = coordinate("r", 0.5, diagonal);
            let focus = (
                attribute("fx").map_or(center.0, |_| coordinate("fx", 0.5, vw)),
                attribute("fy").map_or(center.1, |_| coordinate("fy", 0.5, vh)),
            );
            (radius > 0.0).then_some(Paint::Radial { center, radius, focus, stops, transform })
        }
    }
}

/// Rectangle with elliptical corners.
fn rounded_rect(x: f64, y: f64, w: f64, h: f64, rx: f64, ry: f64) -> Figure {
    if rx <= 0.0 || ry <= 0.0 {
        return Figure::rect(x, y, w, h);
    }
    const K: f64 = 0.552_284_749_830_793_4;
    let (kx, ky) = (rx * K, ry * K);
    let (r, b) = (x + w, y + h);
    Figure {
        start: (x + rx, y),
        segments: vec![
            Segment::Line((r - rx, y)),
            Segment::Cubic((r - rx + kx, y), (r, y + ry - ky), (r, y + ry)),
            Segment::Line((r, b - ry)),
            Segment::Cubic((r, b - ry + ky), (r - rx + kx, b), (r - rx, b)),
            Segment::Line((x + rx, b)),
            Segment::Cubic((x + rx - kx, b), (x, b - ry + ky), (x, b - ry)),
            Segment::Line((x, y + ry)),
            Segment::Cubic((x, y + ry - ky), (x + rx - kx, y), (x + rx, y)),
        ],
        closed: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ops(source: &str) -> Vec<DrawOp> {
        let svg = SvgImage::parse(source).unwrap();
        let size = svg.size();
        svg.operations(0.0, 0.0, size.width as f64, size.height as f64)
    }

    #[test]
    fn sizes_and_viewbox() {
        let svg = SvgImage::parse(r#"<svg viewBox="0 0 40 20"/>"#).unwrap();
        assert_eq!(svg.size(), Size::new(40, 20));
        let svg = SvgImage::parse(r#"<svg width="80" viewBox="0 0 40 20"/>"#).unwrap();
        assert_eq!(svg.size(), Size::new(80, 40), "altura pela proporção");
        let svg = SvgImage::parse(r#"<svg width="1in" height="10mm"/>"#).unwrap();
        assert_eq!(svg.size(), Size::new(96, 38));
        assert!(SvgImage::parse("<html/>").is_err());
        let meet = viewbox_transform((0.0, 0.0, 10.0, 10.0), 40.0, 20.0, "");
        assert_eq!(meet.apply((0.0, 0.0)), (10.0, 0.0), "centralizado na horizontal");
        assert_eq!(meet.apply((10.0, 10.0)), (30.0, 20.0));
        let min = viewbox_transform((0.0, 0.0, 10.0, 10.0), 40.0, 20.0, "xMinYMin meet");
        assert_eq!(min.apply((0.0, 0.0)), (0.0, 0.0));
        let none = viewbox_transform((5.0, 0.0, 10.0, 10.0), 40.0, 20.0, "none");
        assert_eq!(none.apply((15.0, 10.0)), (40.0, 20.0));
        let slice = viewbox_transform((0.0, 0.0, 10.0, 10.0), 40.0, 20.0, "xMidYMid slice");
        assert_eq!(slice.apply((0.0, 5.0)), (0.0, 10.0));
    }

    #[test]
    fn styles_cascade_and_inherit() {
        let ops = ops(r##"<svg viewBox="0 0 10 10">
            <style>.b { fill: blue } #r { stroke: green }</style>
            <g fill="red" stroke-width="2" opacity="0.5">
              <rect width="5" height="5"/>
              <rect id="r" class="b" width="5" height="5" style="fill-opacity:.5"/>
              <circle r="2" fill="none" stroke="currentColor"/>
              <rect width="1" height="1" display="none"/>
              <rect width="1" height="1" visibility="hidden"/>
              <line x2="5" fill="red"/>
            </g>
        </svg>"##);
        assert_eq!(ops.len(), 4, "{ops:#?}");
        match &ops[0] {
            DrawOp::Fill { paint, opacity, .. } => {
                assert_eq!(*paint, Paint::Solid(Color::rgb(255, 0, 0)));
                assert_eq!(*opacity, 0.5, "opacidade do grupo");
            }
            other => panic!("{other:?}"),
        }
        match (&ops[1], &ops[2]) {
            (DrawOp::Fill { paint, opacity, .. }, DrawOp::Stroke { paint: stroke, stroke: s, .. }) => {
                assert_eq!(*paint, Paint::Solid(Color::rgb(0, 0, 255)), "classe vence o atributo herdado");
                assert_eq!(*opacity, 0.25);
                assert_eq!(*stroke, Paint::Solid(Color::rgb(0, 128, 0)), "regra por id");
                assert_eq!(s.width, 2.0, "espessura herdada");
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(&ops[3], DrawOp::Stroke { paint: Paint::Solid(Color::BLACK), .. }), "currentColor");
    }

    #[test]
    fn gradients_use_and_transforms() {
        let ops = ops(r##"<svg viewBox="0 0 100 100" xmlns:xlink="http://www.w3.org/1999/xlink">
            <defs>
              <linearGradient id="base"><stop offset="0" stop-color="#000"/><stop offset="100%" stop-color="#fff" stop-opacity=".5"/></linearGradient>
              <linearGradient id="g" xlink:href="#base" x2="0" y2="1"/>
              <radialGradient id="r" gradientUnits="userSpaceOnUse" cx="50" cy="50" r="40"><stop stop-color="red"/><stop offset="1" stop-color="blue"/></radialGradient>
              <rect id="box" width="10" height="20"/>
            </defs>
            <rect x="10" y="10" width="40" height="20" fill="url(#g)"/>
            <circle cx="50" cy="50" r="40" fill="url(#r)"/>
            <use href="#box" x="5" y="6" transform="scale(2)" fill="url(#missing) orange"/>
        </svg>"##);
        assert_eq!(ops.len(), 3);
        match &ops[0] {
            DrawOp::Fill { paint: Paint::Linear { start, end, stops, transform, .. }, .. } => {
                assert_eq!((*start, *end), ((0.0, 0.0), (0.0, 1.0)), "x2/y2 próprios, paradas herdadas");
                assert_eq!(stops[1].1.a, 128);
                assert_eq!(transform.apply((1.0, 1.0)), (50.0, 30.0), "caixa delimitadora do retângulo");
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(&ops[1], DrawOp::Fill { paint: Paint::Radial { radius, .. }, .. } if *radius == 40.0));
        match &ops[2] {
            DrawOp::Fill { paint, transform, figures, .. } => {
                assert_eq!(*paint, Paint::Solid(Color::rgb(255, 165, 0)), "url ausente usa a cor reserva");
                assert_eq!(transform.apply(figures[0].start), (10.0, 12.0), "scale(2) e depois x/y do use");
            }
            other => panic!("{other:?}"),
        }
    }
}
