//! Drawing/rendering with three selectable backends.
//!
//! | Backend | Entry type | Use |
//! |---|---|---|
//! | GDI | [`Canvas`] | Simple drawing, native text, printing; always available. |
//! | GDI+ | [`PlusGraphics`] via [`Canvas::with_gdiplus`] | Antialiasing, transparency, PNG/JPEG. |
//! | Direct2D + DirectWrite | [`Direct2DSurface`] / [`D2DContext`] | Hardware-accelerated animation/vectors. |

pub mod direct2d;
pub mod font;
pub mod gdi;
pub mod gdiplus;
pub mod svg;
pub mod vector;

pub use direct2d::{D2DContext, Direct2DSurface};
pub use font::Font;
pub use gdi::{Bitmap, Canvas, GradientDirection, Path, PathSegment, Pen, PenStyle, TextAlign};
pub use gdiplus::{PlusGraphics, PlusImage};
pub use svg::SvgImage;
pub use vector::{Figure, LineCap, LineJoin, Matrix, Paint, Segment, Spread, Stroke};
