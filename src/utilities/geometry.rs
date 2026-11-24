//! Geometry/color types throughout public API.

use crate::ffi::{COLORREF, RECT};

/// Integer-coordinate point.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: i32,
    /// Vertical coordinate.
    pub y: i32,
}

impl Point {
    /// Creates point.
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Size (width/height).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Size {
    /// Width.
    pub width: i32,
    /// Height.
    pub height: i32,
}

impl Size {
    /// Creates size.
    pub const fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }
}

/// Rectangle by origin/size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rect {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub width: i32,
    /// Height.
    pub height: i32,
}

impl Rect {
    /// Creates rectangle.
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self { x, y, width, height }
    }

    /// Creates from left/top/right/bottom edges.
    pub const fn from_ltrb(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self { x: left, y: top, width: right - left, height: bottom - top }
    }

    /// Exclusive right edge.
    pub const fn right(&self) -> i32 {
        self.x + self.width
    }

    /// Exclusive bottom edge.
    pub const fn bottom(&self) -> i32 {
        self.y + self.height
    }

    /// Rectangle origin.
    pub const fn position(&self) -> Point {
        Point::new(self.x, self.y)
    }

    /// Rectangle size.
    pub const fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }

    /// Whether point is inside rectangle.
    pub const fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x < self.right() && p.y >= self.y && p.y < self.bottom()
    }

    /// Insets by margins, never producing negative size.
    pub fn inset(&self, t: Thickness) -> Rect {
        Rect {
            x: self.x + t.left,
            y: self.y + t.top,
            width: (self.width - t.left - t.right).max(0),
            height: (self.height - t.top - t.bottom).max(0),
        }
    }

    pub(crate) fn from_native(r: RECT) -> Self {
        Self::from_ltrb(r.left, r.top, r.right, r.bottom)
    }

    pub(crate) fn to_native(self) -> RECT {
        RECT { left: self.x, top: self.y, right: self.right(), bottom: self.bottom() }
    }
}

/// Per-side thickness for margins/padding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Thickness {
    /// Left side.
    pub left: i32,
    /// Top side.
    pub top: i32,
    /// Right side.
    pub right: i32,
    /// Bottom side.
    pub bottom: i32,
}

impl Thickness {
    /// Same thickness on all sides.
    pub const fn all(value: i32) -> Self {
        Self { left: value, top: value, right: value, bottom: value }
    }

    /// Horizontal left/right and vertical top/bottom thickness.
    pub const fn symmetric(horizontal: i32, vertical: i32) -> Self {
        Self { left: horizontal, top: vertical, right: horizontal, bottom: vertical }
    }

    /// Individual side thickness.
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self { left, top, right, bottom }
    }

    /// Horizontal thickness sum.
    pub const fn horizontal(&self) -> i32 {
        self.left + self.right
    }

    /// Vertical thickness sum.
    pub const fn vertical(&self) -> i32 {
        self.top + self.bottom
    }
}

/// RGBA color, 8 bits/channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Opacity (255 = opaque).
    pub a: u8,
}

impl Color {
    /// Black.
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    /// White.
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    /// Red.
    pub const RED: Color = Color::rgb(220, 38, 38);
    /// Green.
    pub const GREEN: Color = Color::rgb(22, 163, 74);
    /// Blue.
    pub const BLUE: Color = Color::rgb(37, 99, 235);
    /// Yellow.
    pub const YELLOW: Color = Color::rgb(234, 179, 8);
    /// Medium gray.
    pub const GRAY: Color = Color::rgb(128, 128, 128);
    /// Fully transparent.
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);

    /// Opaque color.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Color with alpha.
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Opaque color from `0xRRGGBB`.
    pub const fn hex(value: u32) -> Self {
        Self::rgb((value >> 16) as u8, (value >> 8) as u8, value as u8)
    }

    /// Same color with another opacity.
    pub const fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }

    /// Approximate perceptual luminance, 0.0 black to 1.0 white.
    pub fn luminance(&self) -> f32 {
        (0.299 * self.r as f32 + 0.587 * self.g as f32 + 0.114 * self.b as f32) / 255.0
    }

    /// Linear color blend: `t` 0 returns self, 1 returns other.
    pub fn mix(self, other: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        let lerp = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        Color::rgba(lerp(self.r, other.r), lerp(self.g, other.g), lerp(self.b, other.b), lerp(self.a, other.a))
    }

    /// Win32 COLORREF (0x00BBGGRR), discarding alpha.
    pub const fn to_colorref(self) -> COLORREF {
        crate::ffi::rgb(self.r, self.g, self.b)
    }

    /// Creates from Win32 COLORREF.
    pub const fn from_colorref(value: COLORREF) -> Self {
        Self::rgb(value as u8, (value >> 8) as u8, (value >> 16) as u8)
    }

    /// GDI+ ARGB (0xAARRGGBB).
    pub const fn to_argb(self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | self.b as u32
    }
}

/// Reference DPI where one logical unit equals one pixel.
pub const BASE_DPI: u32 = 96;

/// Converts 96-DPI logical units to physical pixels at given DPI.
pub fn to_pixels(logical: i32, dpi: u32) -> i32 {
    ((logical as i64 * dpi as i64 + (BASE_DPI as i64 / 2) * logical.signum() as i64) / BASE_DPI as i64) as i32
}

/// Converts physical pixels to 96-DPI logical units at given DPI.
pub fn to_logical(pixels: i32, dpi: u32) -> i32 {
    let dpi = dpi.max(1) as i64;
    ((pixels as i64 * BASE_DPI as i64 + (dpi / 2) * pixels.signum() as i64) / dpi) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_edges_and_inset() {
        let r = Rect::new(10, 20, 100, 50);
        assert_eq!((r.right(), r.bottom()), (110, 70));
        assert!(r.contains(Point::new(10, 20)));
        assert!(!r.contains(Point::new(110, 20)));
        assert_eq!(r.inset(Thickness::all(5)), Rect::new(15, 25, 90, 40));
        assert_eq!(r.inset(Thickness::all(500)).size(), Size::new(0, 0));
        assert_eq!(Rect::from_native(r.to_native()), r);
    }

    #[test]
    fn color_conversions() {
        let c = Color::hex(0x336699);
        assert_eq!((c.r, c.g, c.b, c.a), (0x33, 0x66, 0x99, 255));
        assert_eq!(c.to_colorref(), 0x0099_6633);
        assert_eq!(Color::from_colorref(c.to_colorref()), c);
        assert_eq!(c.with_alpha(0x80).to_argb(), 0x8033_6699);
        assert_eq!(Color::BLACK.mix(Color::WHITE, 0.5), Color::rgb(128, 128, 128));
        assert!(Color::WHITE.luminance() > 0.99 && Color::BLACK.luminance() < 0.01);
    }

    #[test]
    fn dpi_scaling_round_trips() {
        assert_eq!(to_pixels(100, 96), 100);
        assert_eq!(to_pixels(100, 144), 150);
        assert_eq!(to_pixels(-100, 144), -150);
        assert_eq!(to_pixels(7, 120), 9);
        assert_eq!(to_logical(150, 144), 100);
        for dpi in [96, 120, 144, 192, 240] {
            for v in [0, 1, 8, 23, 150, 801] {
                assert_eq!(to_logical(to_pixels(v, dpi), dpi), v, "dpi {dpi} valor {v}");
            }
        }
    }
}
