//! Native drag/drop through Windows OLE.
//!
//! Three components:
//!
//! * [`start_drag`] / [`start_drag_with_image`] drag text, HTML, images, files or custom formats **outside** the
//!   application to Explorer, Word, browsers or other windows;
//! * [`accept_drops`] makes any control a **target** for other programs or the application, with system drag image and
//!   copy/move cursor;
//! * **internal list/tree reordering**, item image following cursor, accent insertion line, automatic edge scrolling
//!   and Esc cancellation. Enable via [`ListBox::set_reorderable`](crate::controls::ListBox::set_reorderable),
//!   [`ListView::set_reorderable`](crate::controls::ListView::set_reorderable),
//!   [`DataGrid::set_reorderable`](crate::controls::DataGrid::set_reorderable),
//!   [`TreeView::set_drag_enabled`](crate::controls::TreeView::set_drag_enabled). With `on_drag_out`, dragging outside
//!   becomes ordinary OLE dragging, e.g. item text dropped in Notepad.
//!
//! ```no_run
//! # use arqrustui::prelude::*;
//! # fn demo(window: &Window, zone: &Panel, list: &ListBox) -> GuiResult<()> {
//! // Target accepting files/text.
//! accept_drops(zone, DropHandler::new()
//!     .on_over(|data| if data.has_files() || data.has_text() { DropEffect::Copy } else { DropEffect::None })
//!     .on_drop(|data, _effect| {
//!         for file in data.files() { println!("file: {}", file.display()); }
//!         if let Some(text) = data.text() { println!("text: {text}"); }
//!     }))?;
//!
//! // Reorderable list whose items can also be dragged outside.
//! list.set_reorderable(true);
//! let source = list.clone();
//! list.on_drag_out(move |index| source.item_text(index).map(|t| ClipboardContent::new().text(&t)));
//! # Ok(()) }
//! ```

use crate::ffi::comctl32::*;
use crate::ffi::gdi32::*;
use crate::ffi::ole32::*;
use crate::ffi::user32::*;
use crate::ffi::*;

mod reorder;
mod source;
mod target;

pub(crate) use reorder::*;
pub use source::*;
pub use target::*;

// ---------------------------------------------------------------------------
// Efeitos
// ---------------------------------------------------------------------------

/// Drag/drop result or intent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DropEffect {
    /// No action; target rejects data.
    None,
    /// Target copies data.
    Copy,
    /// Target moves data; source should delete originals.
    Move,
    /// Target creates a data shortcut/link.
    Link,
}

impl DropEffect {
    fn bits(self) -> DWORD {
        match self {
            DropEffect::None => DROPEFFECT_NONE,
            DropEffect::Copy => DROPEFFECT_COPY,
            DropEffect::Move => DROPEFFECT_MOVE,
            DropEffect::Link => DROPEFFECT_LINK,
        }
    }

    fn mask(effects: &[DropEffect]) -> DWORD {
        effects.iter().fold(0, |mask, effect| mask | effect.bits())
    }

    /// Single system effect, preferring move like Explorer.
    fn from_bits(bits: DWORD) -> DropEffect {
        if bits & DROPEFFECT_MOVE != 0 {
            DropEffect::Move
        } else if bits & DROPEFFECT_COPY != 0 {
            DropEffect::Copy
        } else if bits & DROPEFFECT_LINK != 0 {
            DropEffect::Link
        } else {
            DropEffect::None
        }
    }
}

/// `DROPEFFECT_*` mask from effects.
pub(crate) fn effects_mask(effects: &[DropEffect]) -> DWORD {
    DropEffect::mask(effects)
}

/// Restricts desired effect to allowed source effects.
pub(crate) fn negotiate(wanted: DropEffect, allowed: DWORD) -> DropEffect {
    if wanted == DropEffect::None || allowed & wanted.bits() != 0 {
        return wanted;
    }
    for fallback in [DropEffect::Copy, DropEffect::Move, DropEffect::Link] {
        if allowed & fallback.bits() != 0 {
            return fallback;
        }
    }
    DropEffect::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effects_negotiate_with_what_the_source_allows() {
        assert_eq!(negotiate(DropEffect::Move, DROPEFFECT_COPY | DROPEFFECT_MOVE), DropEffect::Move);
        assert_eq!(negotiate(DropEffect::Move, DROPEFFECT_COPY), DropEffect::Copy);
        assert_eq!(negotiate(DropEffect::Link, DROPEFFECT_MOVE), DropEffect::Move);
        assert_eq!(negotiate(DropEffect::Copy, 0), DropEffect::None);
        assert_eq!(negotiate(DropEffect::None, DROPEFFECT_COPY), DropEffect::None);
        assert_eq!(DropEffect::mask(&[DropEffect::Copy, DropEffect::Link]), DROPEFFECT_COPY | DROPEFFECT_LINK);
        assert_eq!(DropEffect::from_bits(DROPEFFECT_COPY | DROPEFFECT_MOVE), DropEffect::Move);
        assert_eq!(DropEffect::from_bits(0), DropEffect::None);
    }

    #[test]
    fn drop_source_follows_buttons_and_escape() {
        assert_eq!(continue_drag(true, MK_LBUTTON as u32), DRAGDROP_S_CANCEL);
        assert_eq!(continue_drag(false, 0), DRAGDROP_S_DROP);
        assert_eq!(continue_drag(false, MK_LBUTTON as u32), S_OK);
    }

    #[test]
    fn slots_map_to_final_indices() {
        // Item 1 de [a b c d] solto antes de "d" (posição 3) termina no índice 2.
        assert_eq!(slot_to_index(1, 3), 2);
        assert_eq!(slot_to_index(3, 0), 0);
        assert_eq!(slot_to_index(0, 4), 3);
        assert_eq!(slot_to_index(2, 2), 2);
    }

    #[test]
    fn block_moves_reach_the_wanted_order() {
        let apply = |count: usize, items: &[usize], slot: usize| {
            let mut list: Vec<usize> = (0..count).collect();
            for (from, to) in block_moves(count, items, slot) {
                let value = list.remove(from);
                list.insert(to, value);
            }
            list
        };
        assert_eq!(apply(6, &[0, 1], 5), [2, 3, 4, 0, 1, 5]);
        assert_eq!(apply(5, &[3, 1], 0), [1, 3, 0, 2, 4]);
        assert_eq!(apply(5, &[0, 2], 5), [1, 3, 4, 0, 2]);
        assert_eq!(apply(4, &[1, 2], 4), [0, 3, 1, 2]);
        assert!(block_moves(4, &[1, 2], 1).is_empty(), "bloco já no lugar");
        assert!(block_moves(4, &[1, 2], 3).is_empty());
    }

    #[test]
    fn tree_zones_split_rows_in_quarters() {
        assert_eq!(tree_zone(0, 20), 1);
        assert_eq!(tree_zone(4, 20), 1);
        assert_eq!(tree_zone(5, 20), 0);
        assert_eq!(tree_zone(14, 20), 0);
        assert_eq!(tree_zone(15, 20), 2);
        assert_eq!(tree_zone(19, 20), 2);
    }

    #[test]
    fn lparam_points_round_trip_negative_coordinates() {
        let point = POINT { x: -12, y: 345 };
        let back = lparam_point(point_lparam(point));
        assert_eq!((back.x, back.y), (-12, 345));
    }
}
