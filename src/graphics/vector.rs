//! Floating-point vector primitives (affine matrix, curved figures, solid/gradient brushes, stroke) and GDI+ drawing,
//! used by [`SvgImage`](super::SvgImage) and [`Chart`](crate::controls::Chart).

use super::gdiplus::PlusGraphics;
use crate::ffi::gdiplus::*;
use crate::utilities::Color;
use std::ffi::c_void;

/// 2D affine matrix: `x' = a·x + c·y + e`, `y' = b·x + d·y + f` (SVG order).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix {
    /// Coefficients `[a, b, c, d, e, f]`.
    pub m: [f64; 6],
}

impl Default for Matrix {
    fn default() -> Self {
        Matrix::IDENTITY
    }
}

impl Matrix {
    /// Identity.
    pub const IDENTITY: Matrix = Matrix { m: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0] };

    /// Translation.
    pub fn translate(x: f64, y: f64) -> Matrix {
        Matrix { m: [1.0, 0.0, 0.0, 1.0, x, y] }
    }

    /// Scale.
    pub fn scale(x: f64, y: f64) -> Matrix {
        Matrix { m: [x, 0.0, 0.0, y, 0.0, 0.0] }
    }

    /// Rotation in degrees, clockwise on screen as in SVG.
    pub fn rotate(degrees: f64) -> Matrix {
        let (sin, cos) = degrees.to_radians().sin_cos();
        Matrix { m: [cos, sin, -sin, cos, 0.0, 0.0] }
    }

    /// `self` applied after `inner` (`self × inner`).
    pub fn then(&self, outer: &Matrix) -> Matrix {
        outer.multiply(self)
    }

    /// Product `self × other`, applying `other` first.
    pub fn multiply(&self, other: &Matrix) -> Matrix {
        let [a, b, c, d, e, f] = self.m;
        let [a2, b2, c2, d2, e2, f2] = other.m;
        Matrix {
            m: [
                a * a2 + c * b2,
                b * a2 + d * b2,
                a * c2 + c * d2,
                b * c2 + d * d2,
                a * e2 + c * f2 + e,
                b * e2 + d * f2 + f,
            ],
        }
    }

    /// Transforms a point.
    pub fn apply(&self, (x, y): (f64, f64)) -> (f64, f64) {
        let [a, b, c, d, e, f] = self.m;
        (a * x + c * y + e, b * x + d * y + f)
    }

    /// Inverse (`None` if degenerate).
    pub fn inverse(&self) -> Option<Matrix> {
        let [a, b, c, d, e, f] = self.m;
        let det = a * d - b * c;
        if det.abs() < 1e-12 {
            return None;
        }
        Some(Matrix { m: [d / det, -b / det, -c / det, a / det, (c * f - d * e) / det, (b * e - a * f) / det] })
    }

    /// Whether angles/aspect ratios are preserved (rotation, uniform scale, reflection, translation); transformed
    /// strokes retain uniform width.
    pub fn is_similarity(&self) -> bool {
        let [a, b, c, d, ..] = self.m;
        let (x, y) = (a * a + b * b, c * c + d * d);
        (x - y).abs() <= 1e-9 * x.max(y).max(1e-12) && (a * c + b * d).abs() <= 1e-9 * x.max(1e-12)
    }

    /// Average scale for stroke widths.
    pub fn mean_scale(&self) -> f64 {
        let [a, b, c, d, ..] = self.m;
        (a * d - b * c).abs().sqrt()
    }
}

/// Figure segment.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Segment {
    /// Line to point.
    Line((f64, f64)),
    /// Cubic Bézier with two control points and endpoint.
    Cubic((f64, f64), (f64, f64), (f64, f64)),
}

/// Figure/subpath: start point, segments and closed flag.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Figure {
    /// Start point.
    pub start: (f64, f64),
    /// Segments.
    pub segments: Vec<Segment>,
    /// Closed, returning to start.
    pub closed: bool,
}

impl Figure {
    /// Figure starting at `start`.
    pub fn new(start: (f64, f64)) -> Figure {
        Figure { start, segments: Vec::new(), closed: false }
    }

    /// Current point at the last segment's end.
    pub fn end(&self) -> (f64, f64) {
        match self.segments.last() {
            Some(Segment::Line(p)) | Some(Segment::Cubic(_, _, p)) => *p,
            None => self.start,
        }
    }

    /// Rectangle as closed figure.
    pub fn rect(x: f64, y: f64, w: f64, h: f64) -> Figure {
        Figure {
            start: (x, y),
            segments: vec![Segment::Line((x + w, y)), Segment::Line((x + w, y + h)), Segment::Line((x, y + h))],
            closed: true,
        }
    }

    /// Ellipse as four Bézier curves.
    pub fn ellipse(cx: f64, cy: f64, rx: f64, ry: f64) -> Figure {
        const K: f64 = 0.552_284_749_830_793_4;
        let (kx, ky) = (rx * K, ry * K);
        Figure {
            start: (cx + rx, cy),
            segments: vec![
                Segment::Cubic((cx + rx, cy + ky), (cx + kx, cy + ry), (cx, cy + ry)),
                Segment::Cubic((cx - kx, cy + ry), (cx - rx, cy + ky), (cx - rx, cy)),
                Segment::Cubic((cx - rx, cy - ky), (cx - kx, cy - ry), (cx, cy - ry)),
                Segment::Cubic((cx + kx, cy - ry), (cx + rx, cy - ky), (cx + rx, cy)),
            ],
            closed: true,
        }
    }

    /// Pie sector between angles in degrees (0 = right, clockwise).
    pub fn pie(cx: f64, cy: f64, radius: f64, start_deg: f64, sweep_deg: f64) -> Figure {
        let mut figure = Figure::new((cx, cy));
        let (start, sweep) = (start_deg.to_radians(), sweep_deg.to_radians());
        figure.segments.push(Segment::Line((cx + radius * start.cos(), cy + radius * start.sin())));
        arc_segments(&mut figure, cx, cy, radius, radius, 0.0, start, sweep);
        figure.closed = true;
        figure
    }
}

/// Appends an elliptical arc to `figure` (center, radii, radian rotation, start/sweep radians) as Bézier curves of at
/// most 90°.
#[allow(clippy::too_many_arguments)]
pub(crate) fn arc_segments(figure: &mut Figure, cx: f64, cy: f64, rx: f64, ry: f64, phi: f64, start: f64, sweep: f64) {
    let pieces = (sweep.abs() / std::f64::consts::FRAC_PI_2).ceil().max(1.0) as usize;
    let step = sweep / pieces as f64;
    let (sin_phi, cos_phi) = phi.sin_cos();
    let point = |t: f64| {
        let (x, y) = (rx * t.cos(), ry * t.sin());
        (cx + x * cos_phi - y * sin_phi, cy + x * sin_phi + y * cos_phi)
    };
    let derivative = |t: f64| {
        let (x, y) = (-rx * t.sin(), ry * t.cos());
        (x * cos_phi - y * sin_phi, x * sin_phi + y * cos_phi)
    };
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    let mut t = start;
    for _ in 0..pieces {
        let (p0, p3) = (point(t), point(t + step));
        let (d0, d3) = (derivative(t), derivative(t + step));
        figure.segments.push(Segment::Cubic(
            (p0.0 + k * d0.0, p0.1 + k * d0.1),
            (p3.0 - k * d3.0, p3.1 - k * d3.1),
            p3,
        ));
        t += step;
    }
}

/// Figure bounds `(min_x, min_y, max_x, max_y)`, including control points.
pub fn bounds(figures: &[Figure]) -> Option<(f64, f64, f64, f64)> {
    let mut points = figures.iter().flat_map(|f| {
        std::iter::once(f.start).chain(f.segments.iter().flat_map(|s| match *s {
            Segment::Line(p) => vec![p],
            Segment::Cubic(a, b, c) => vec![a, b, c],
        }))
    });
    let first = points.next()?;
    Some(points.fold((first.0, first.1, first.0, first.1), |(x0, y0, x1, y1), (x, y)| {
        (x0.min(x), y0.min(y), x1.max(x), y1.max(y))
    }))
}

/// Gradient extension beyond stops.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Spread {
    /// Repeats endpoint color.
    #[default]
    Pad,
    /// Repeats gradient.
    Repeat,
    /// Reflects gradient.
    Reflect,
}

/// Fill/stroke paint.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Paint {
    /// Solid color.
    Solid(Color),
    /// Linear gradient from `(x1, y1)` to `(x2, y2)` in gradient space.
    Linear {
        /// Start.
        start: (f64, f64),
        /// End.
        end: (f64, f64),
        /// Stops `(position 0..1, color)`.
        stops: Vec<(f64, Color)>,
        /// Gradient space → drawing space.
        transform: Matrix,
        /// Spread mode.
        spread: Spread,
    },
    /// Radial gradient (center, radius, focus) in gradient space.
    Radial {
        /// Center.
        center: (f64, f64),
        /// Radius.
        radius: f64,
        /// Focus.
        focus: (f64, f64),
        /// Stops `(position 0..1, color)`.
        stops: Vec<(f64, Color)>,
        /// Gradient space → drawing space.
        transform: Matrix,
    },
}

/// Stroke segment join.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum LineJoin {
    /// Miter.
    #[default]
    Miter,
    /// Bevel.
    Bevel,
    /// Round.
    Round,
}

/// Open stroke cap.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum LineCap {
    /// Flat at the endpoint.
    #[default]
    Butt,
    /// Square beyond the endpoint.
    Square,
    /// Round.
    Round,
}

/// Stroke style.
#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    /// Width in drawing units.
    pub width: f64,
    /// Join.
    pub join: LineJoin,
    /// Cap.
    pub cap: LineCap,
    /// Miter limit.
    pub miter_limit: f64,
    /// Alternating dash lengths in drawing units; empty means solid.
    pub dashes: Vec<f64>,
}

impl Default for Stroke {
    fn default() -> Self {
        Stroke { width: 1.0, join: LineJoin::Miter, cap: LineCap::Butt, miter_limit: 4.0, dashes: Vec::new() }
    }
}

/// GDI+ normalized stops: increasing positions from 0 to 1.
fn normalized_stops(stops: &[(f64, Color)], opacity: f64) -> Vec<(f32, u32)> {
    let fade = |c: Color| Color { a: (c.a as f64 * opacity).round().clamp(0.0, 255.0) as u8, ..c }.to_argb();
    let mut out: Vec<(f32, u32)> = Vec::new();
    let mut last = 0.0f64;
    for &(offset, color) in stops {
        let offset = offset.clamp(last, 1.0);
        last = offset;
        out.push((offset as f32, fade(color)));
    }
    if out.is_empty() {
        out.push((0.0, 0));
    }
    if out[0].0 > 0.0 {
        out.insert(0, (0.0, out[0].1));
    }
    if out[out.len() - 1].0 < 1.0 {
        let color = out[out.len() - 1].1;
        out.push((1.0, color));
    }
    if out.len() == 1 {
        out.push((1.0, out[0].1));
    }
    out
}

/// Extended axis and repositioned stops for a `Pad` linear gradient.
pub(crate) type PaddedLinear = ((f64, f64), (f64, f64), Vec<(f64, Color)>);

/// For `Pad`, extends the gradient axis to cover gradient-space `area` and repositions stops because GDI+ repeats
/// outside the axis.
pub(crate) fn pad_linear(
    start: (f64, f64),
    end: (f64, f64),
    stops: &[(f64, Color)],
    area: (f64, f64, f64, f64),
) -> PaddedLinear {
    let (dx, dy) = (end.0 - start.0, end.1 - start.1);
    let length = dx * dx + dy * dy;
    if length < 1e-12 || stops.is_empty() {
        return (start, end, stops.to_vec());
    }
    let project = |(x, y): (f64, f64)| ((x - start.0) * dx + (y - start.1) * dy) / length;
    let corners = [(area.0, area.1), (area.2, area.1), (area.0, area.3), (area.2, area.3)];
    let tmin = corners.iter().map(|&p| project(p)).fold(0.0f64, f64::min);
    let tmax = corners.iter().map(|&p| project(p)).fold(1.0f64, f64::max);
    let span = tmax - tmin;
    let remap = |t: f64| (t - tmin) / span;
    let mut out = vec![(0.0, stops[0].1)];
    out.extend(stops.iter().map(|&(t, c)| (remap(t.clamp(0.0, 1.0)), c)));
    out.push((1.0, stops[stops.len() - 1].1));
    let at = |t: f64| (start.0 + dx * t, start.1 + dy * t);
    (at(tmin), at(tmax), out)
}

/// GDI+ primitive rendering.
pub(crate) struct VectorPainter<'a> {
    pub graphics: &'a PlusGraphics,
}

/// GDI+ resource deleted on drop.
struct Owned(*mut c_void, unsafe extern "system" fn(*mut c_void) -> GpStatus);

impl Drop for Owned {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: objeto criado por este módulo, apagado uma única vez pela função certa.
            unsafe { (self.1)(self.0) };
        }
    }
}

fn gp_matrix(m: &Matrix) -> Option<Owned> {
    let [a, b, c, d, e, f] = m.m;
    let mut matrix = std::ptr::null_mut();
    // SAFETY: ponteiro de saída válido; a matriz é apagada pelo `Owned`.
    let status = unsafe { GdipCreateMatrix2(a as f32, b as f32, c as f32, d as f32, e as f32, f as f32, &mut matrix) };
    (status == 0).then_some(Owned(matrix, GdipDeleteMatrix))
}

fn gp_path(figures: &[Figure], even_odd: bool) -> Option<Owned> {
    let mut path = std::ptr::null_mut();
    let mode = if even_odd { FILL_MODE_ALTERNATE } else { FILL_MODE_WINDING };
    // SAFETY: ponteiro de saída válido; o caminho é apagado pelo `Owned`.
    if unsafe { GdipCreatePath(mode, &mut path) } != 0 {
        return None;
    }
    let owned = Owned(path, GdipDeletePath);
    for figure in figures {
        // SAFETY: caminho válido; coordenadas finitas.
        unsafe {
            GdipStartPathFigure(path);
            let mut current = figure.start;
            for segment in &figure.segments {
                match *segment {
                    Segment::Line(p) => {
                        GdipAddPathLine(path, current.0 as f32, current.1 as f32, p.0 as f32, p.1 as f32);
                        current = p;
                    }
                    Segment::Cubic(a, b, c) => {
                        GdipAddPathBezier(
                            path,
                            current.0 as f32,
                            current.1 as f32,
                            a.0 as f32,
                            a.1 as f32,
                            b.0 as f32,
                            b.1 as f32,
                            c.0 as f32,
                            c.1 as f32,
                        );
                        current = c;
                    }
                }
            }
            if figure.closed {
                GdipClosePathFigure(path);
            }
        }
    }
    Some(owned)
}

impl VectorPainter<'_> {
    fn raw(&self) -> *mut c_void {
        self.graphics.raw()
    }

    /// Runs `draw` with appended `transform`, restoring afterward.
    pub fn with_transform(&self, transform: &Matrix, draw: impl FnOnce()) {
        let mut state = 0u32;
        // SAFETY: contexto válido; o estado salvo é restaurado abaixo.
        unsafe { GdipSaveGraphics(self.raw(), &mut state) };
        if let Some(matrix) = gp_matrix(transform) {
            // SAFETY: contexto e matriz válidos.
            unsafe { GdipMultiplyWorldTransform(self.raw(), matrix.0, MATRIX_ORDER_PREPEND) };
            draw();
        }
        // SAFETY: estado salvo acima neste mesmo contexto.
        unsafe { GdipRestoreGraphics(self.raw(), state) };
    }

    /// Runs `draw` clipped to a drawing-coordinate rectangle, restoring afterward.
    pub fn with_clip(&self, rect: crate::utilities::Rect, draw: impl FnOnce()) {
        let mut state = 0u32;
        // SAFETY: contexto válido; o estado salvo é restaurado abaixo.
        unsafe {
            GdipSaveGraphics(self.raw(), &mut state);
            GdipSetClipRect(
                self.raw(),
                rect.x as f32,
                rect.y as f32,
                rect.width as f32,
                rect.height as f32,
                COMBINE_MODE_INTERSECT,
            );
        }
        draw();
        // SAFETY: estado salvo acima neste mesmo contexto.
        unsafe { GdipRestoreGraphics(self.raw(), state) };
    }

    /// Creates a `paint` brush for figures bounded by drawing-space `area`. For radial `Pad`, `outside` receives the
    /// path/color outside the ellipse.
    fn brush(&self, paint: &Paint, opacity: f64, area: (f64, f64, f64, f64)) -> Option<(Owned, Option<(Owned, u32)>)> {
        let mut brush = std::ptr::null_mut();
        match paint {
            Paint::Solid(color) => {
                let a = (color.a as f64 * opacity).round().clamp(0.0, 255.0) as u8;
                // SAFETY: ponteiro de saída válido.
                (unsafe { GdipCreateSolidFill(Color { a, ..*color }.to_argb(), &mut brush) } == 0)
                    .then(|| (Owned(brush, GdipDeleteBrush), None))
            }
            Paint::Linear { start, end, stops, transform, spread } => {
                let inverse = transform.inverse()?;
                let corners =
                    [(area.0, area.1), (area.2, area.1), (area.0, area.3), (area.2, area.3)].map(|p| inverse.apply(p));
                let local =
                    corners.iter().fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |(x0, y0, x1, y1), &(x, y)| {
                        (x0.min(x), y0.min(y), x1.max(x), y1.max(y))
                    });
                let (start, end, stops) = match spread {
                    Spread::Pad => pad_linear(*start, *end, stops, local),
                    _ => (*start, *end, stops.clone()),
                };
                let stops = normalized_stops(&stops, opacity);
                let (p1, p2) =
                    (PointF { x: start.0 as f32, y: start.1 as f32 }, PointF { x: end.0 as f32, y: end.1 as f32 });
                let wrap = if *spread == Spread::Reflect { WRAP_MODE_TILE_FLIP_XY } else { WRAP_MODE_TILE };
                // SAFETY: pontos e paradas locais; o pincel é apagado pelo `Owned`.
                unsafe {
                    if GdipCreateLineBrush(&p1, &p2, stops[0].1, stops[stops.len() - 1].1, wrap, &mut brush) != 0 {
                        return None;
                    }
                    let owned = Owned(brush, GdipDeleteBrush);
                    let (colors, positions): (Vec<u32>, Vec<f32>) = stops.iter().map(|&(p, c)| (c, p)).unzip();
                    GdipSetLinePresetBlend(brush, colors.as_ptr(), positions.as_ptr(), colors.len() as i32);
                    if *transform != Matrix::IDENTITY {
                        let matrix = gp_matrix(transform)?;
                        GdipMultiplyLineTransform(brush, matrix.0, MATRIX_ORDER_APPEND);
                    }
                    Some((owned, None))
                }
            }
            Paint::Radial { center, radius, focus, stops, transform } => {
                let ellipse = Figure::ellipse(center.0, center.1, *radius, *radius);
                let transformed = transform_figures(std::slice::from_ref(&ellipse), transform);
                let path = gp_path(&transformed, false)?;
                let stops = normalized_stops(stops, opacity);
                // GDI+: posição 0 = borda, 1 = centro (o inverso do SVG).
                let (colors, positions): (Vec<u32>, Vec<f32>) = stops.iter().rev().map(|&(p, c)| (c, 1.0 - p)).unzip();
                let focus = transform.apply(*focus);
                // SAFETY: caminho válido; o pincel é apagado pelo `Owned`.
                unsafe {
                    if GdipCreatePathGradientFromPath(path.0, &mut brush) != 0 {
                        return None;
                    }
                    let owned = Owned(brush, GdipDeleteBrush);
                    // Cores do centro e da borda primeiro (implementações que ignoram a
                    // mistura por paradas, como o GDI+ do Wine, ficam com duas cores);
                    // a mistura por paradas definida em seguida tem precedência.
                    GdipSetPathGradientCenterColor(brush, stops[0].1);
                    let mut count = 1i32;
                    GdipSetPathGradientSurroundColorsWithCount(brush, &stops[stops.len() - 1].1, &mut count);
                    GdipSetPathGradientPresetBlend(brush, colors.as_ptr(), positions.as_ptr(), colors.len() as i32);
                    GdipSetPathGradientCenterPoint(brush, &PointF { x: focus.0 as f32, y: focus.1 as f32 });
                    let outside = stops[stops.len() - 1].1;
                    Some((owned, Some((path, outside))))
                }
            }
        }
    }

    /// Fills figures.
    pub fn fill(&self, figures: &[Figure], even_odd: bool, paint: &Paint, opacity: f64) {
        let Some(area) = bounds(figures) else { return };
        let Some(path) = gp_path(figures, even_odd) else { return };
        let Some((brush, outside)) = self.brush(paint, opacity, area) else { return };
        // SAFETY: contexto, caminho e pincéis válidos durante a chamada.
        unsafe {
            if let Some((ellipse, color)) = outside {
                // Fora da elipse do gradiente radial: a cor da última parada (Pad).
                let mut state = 0u32;
                GdipSaveGraphics(self.raw(), &mut state);
                GdipSetClipPath(self.raw(), ellipse.0, COMBINE_MODE_EXCLUDE);
                let mut solid = std::ptr::null_mut();
                if GdipCreateSolidFill(color, &mut solid) == 0 {
                    let solid = Owned(solid, GdipDeleteBrush);
                    GdipFillPath(self.raw(), solid.0, path.0);
                }
                GdipRestoreGraphics(self.raw(), state);
            }
            GdipFillPath(self.raw(), brush.0, path.0);
        }
    }

    /// Strokes figures.
    pub fn stroke(&self, figures: &[Figure], stroke: &Stroke, paint: &Paint, opacity: f64) {
        if stroke.width <= 0.0 {
            return;
        }
        let Some(area) = bounds(figures) else { return };
        let half = stroke.width / 2.0;
        let area = (area.0 - half, area.1 - half, area.2 + half, area.3 + half);
        let Some(path) = gp_path(figures, false) else { return };
        let Some((brush, _)) = self.brush(paint, opacity, area) else { return };
        let mut pen = std::ptr::null_mut();
        // SAFETY: pincel válido; a caneta é apagada pelo `Owned`.
        unsafe {
            if GdipCreatePen2(brush.0, stroke.width as f32, UNIT_PIXEL, &mut pen) != 0 {
                return;
            }
            let pen = Owned(pen, GdipDeletePen);
            GdipSetPenLineJoin(
                pen.0,
                match stroke.join {
                    LineJoin::Miter => LINE_JOIN_MITER,
                    LineJoin::Bevel => LINE_JOIN_BEVEL,
                    LineJoin::Round => LINE_JOIN_ROUND,
                },
            );
            let cap = match stroke.cap {
                LineCap::Butt => LINE_CAP_FLAT,
                LineCap::Square => LINE_CAP_SQUARE,
                LineCap::Round => LINE_CAP_ROUND,
            };
            GdipSetPenStartCap(pen.0, cap);
            GdipSetPenEndCap(pen.0, cap);
            GdipSetPenMiterLimit(pen.0, stroke.miter_limit.max(1.0) as f32);
            if !stroke.dashes.is_empty() && stroke.dashes.iter().any(|&d| d > 0.0) {
                // O GDI+ mede o tracejado em espessuras do traço.
                let mut dashes: Vec<f32> = stroke.dashes.iter().map(|&d| (d / stroke.width).max(0.01) as f32).collect();
                if dashes.len() % 2 == 1 {
                    dashes.extend_from_within(..);
                }
                GdipSetPenDashArray(pen.0, dashes.as_ptr(), dashes.len() as i32);
            }
            GdipDrawPath(self.raw(), pen.0, path.0);
        }
    }
}

impl Paint {
    /// Paint after `m`; gradient space follows the figure.
    pub fn transformed(&self, m: &Matrix) -> Paint {
        match self {
            Paint::Solid(c) => Paint::Solid(*c),
            Paint::Linear { start, end, stops, transform, spread } => Paint::Linear {
                start: *start,
                end: *end,
                stops: stops.clone(),
                transform: m.multiply(transform),
                spread: *spread,
            },
            Paint::Radial { center, radius, focus, stops, transform } => Paint::Radial {
                center: *center,
                radius: *radius,
                focus: *focus,
                stops: stops.clone(),
                transform: m.multiply(transform),
            },
        }
    }
}

impl VectorPainter<'_> {
    /// Fills figures in `transform` space. For similarity transforms (common case), transforms points here and draws
    /// without a GDI+ transform.
    pub fn fill_in(&self, transform: &Matrix, figures: &[Figure], even_odd: bool, paint: &Paint, opacity: f64) {
        if transform.is_similarity() {
            self.fill(&transform_figures(figures, transform), even_odd, &paint.transformed(transform), opacity);
        } else {
            self.with_transform(transform, || self.fill(figures, even_odd, paint, opacity));
        }
    }

    /// Strokes figures in `transform` space. For similarities, scales width/dashes here, independent of GDI+ pen
    /// scaling; otherwise GDI+ transforms the pen.
    pub fn stroke_in(&self, transform: &Matrix, figures: &[Figure], stroke: &Stroke, paint: &Paint, opacity: f64) {
        if transform.is_similarity() {
            let scale = transform.mean_scale();
            let stroke = Stroke {
                width: stroke.width * scale,
                dashes: stroke.dashes.iter().map(|d| d * scale).collect(),
                ..stroke.clone()
            };
            self.stroke(&transform_figures(figures, transform), &stroke, &paint.transformed(transform), opacity);
        } else {
            self.with_transform(transform, || self.stroke(figures, stroke, paint, opacity));
        }
    }
}

/// Figures with all points transformed by the matrix.
pub fn transform_figures(figures: &[Figure], m: &Matrix) -> Vec<Figure> {
    figures
        .iter()
        .map(|f| Figure {
            start: m.apply(f.start),
            segments: f
                .segments
                .iter()
                .map(|s| match *s {
                    Segment::Line(p) => Segment::Line(m.apply(p)),
                    Segment::Cubic(a, b, c) => Segment::Cubic(m.apply(a), m.apply(b), m.apply(c)),
                })
                .collect(),
            closed: f.closed,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
    }

    #[test]
    fn matrices_compose_and_invert() {
        let m = Matrix::translate(10.0, 5.0).multiply(&Matrix::scale(2.0, 3.0));
        assert!(close(m.apply((1.0, 1.0)), (12.0, 8.0)), "escala primeiro, depois translação");
        let back = m.inverse().unwrap();
        assert!(close(back.apply((12.0, 8.0)), (1.0, 1.0)));
        assert!(close(Matrix::rotate(90.0).apply((1.0, 0.0)), (0.0, 1.0)), "90° gira para baixo (y cresce)");
        assert!(Matrix::scale(0.0, 1.0).inverse().is_none());
        assert!((Matrix::scale(2.0, 8.0).mean_scale() - 4.0).abs() < 1e-12);
        assert!(Matrix::rotate(30.0).multiply(&Matrix::scale(3.0, 3.0)).is_similarity());
        assert!(Matrix::scale(-2.0, 2.0).is_similarity(), "espelhamento");
        assert!(!Matrix::scale(2.0, 1.0).is_similarity());
        assert!(!Matrix { m: [1.0, 0.0, 0.5, 1.0, 0.0, 0.0] }.is_similarity(), "inclinação");
        assert_eq!(Matrix::scale(2.0, 2.0).then(&Matrix::translate(1.0, 0.0)).apply((1.0, 1.0)), (3.0, 2.0));
    }

    #[test]
    fn shapes_and_bounds() {
        let e = Figure::ellipse(10.0, 10.0, 5.0, 3.0);
        assert_eq!(e.segments.len(), 4);
        assert!(close(e.end(), (15.0, 10.0)));
        let (x0, y0, x1, y1) = bounds(&[e]).unwrap();
        assert!(
            (x0 - 5.0).abs() < 1e-9 && (x1 - 15.0).abs() < 1e-9 && (y0 - 7.0).abs() < 1e-9 && (y1 - 13.0).abs() < 1e-9
        );
        let pie = Figure::pie(0.0, 0.0, 10.0, 0.0, 90.0);
        assert!(close(pie.end(), (0.0, 10.0)), "um quarto no sentido horário termina embaixo");
        assert!(bounds(&[]).is_none());
    }

    #[test]
    fn pad_extends_the_gradient_axis() {
        let stops = [(0.0, Color::BLACK), (1.0, Color::WHITE)];
        let (a, b, s) = pad_linear((0.0, 0.0), (10.0, 0.0), &stops, (-10.0, 0.0, 20.0, 5.0));
        assert!(close(a, (-10.0, 0.0)) && close(b, (20.0, 0.0)));
        let positions: Vec<f64> = s.iter().map(|p| p.0).collect();
        assert_eq!(positions.len(), 4);
        assert!((positions[1] - 1.0 / 3.0).abs() < 1e-9 && (positions[2] - 2.0 / 3.0).abs() < 1e-9);
        let n = normalized_stops(&[(0.3, Color::BLACK), (0.2, Color::WHITE)], 0.5);
        assert_eq!(n.iter().map(|s| s.0).collect::<Vec<_>>(), vec![0.0, 0.3, 0.3, 1.0], "posições nunca recuam");
        assert_eq!(n[0].1 >> 24, 128, "opacidade aplicada ao alfa");
    }
}
