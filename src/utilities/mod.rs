//! Window-independent geometry, color and string utilities.

pub mod geometry;
pub mod strings;

pub use geometry::{to_logical, to_pixels, Color, Point, Rect, Size, Thickness, BASE_DPI};
pub use strings::{from_wide, to_wide};
