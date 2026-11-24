//! Less common native controls: [`Hyperlink`], [`HeaderControl`], [`Rebar`], [`Pager`], [`HotKeyBox`], [`IpAddressBox`]
//! and [`Animation`].

use super::common::{plain_builder, widget_type, ControlOptions};
use crate::core::native;
use crate::core::widget::{Reflected, Widget, WidgetCore};
use crate::core::{Event, EventKind, GuiError, GuiResult, HandlerId};
use crate::ffi::comctl32::*;
use crate::ffi::user32::*;
use crate::input::{Key, Modifiers, Shortcut};
use crate::utilities::{from_wide, to_logical, to_pixels, to_wide};
use std::net::Ipv4Addr;
use std::path::Path;

// ---- Hyperlink -------------------------------------------------------------------

widget_type!(
    /// Text with one or more clickable links (SysLink control).
    ///
    /// Text uses `<a href="...">label</a>` or `<a id="...">label</a>` markup.
    Hyperlink
);

plain_builder!(
    /// Builder for [`Hyperlink`].
    HyperlinkBuilder
);

impl HyperlinkBuilder {
    /// Creates the control.
    pub fn build(self) -> GuiResult<Hyperlink> {
        let core = self.opts.create("Hyperlink", "SysLink", 0, 0, true, None)?;
        core.set_reflect(|core, reflected| match reflected {
            Reflected::Notify { code, lparam } if *code == NM_CLICK || *code == NM_RETURN => {
                // SAFETY: nestas notificações do SysLink o LPARAM aponta para um NMLINK válido.
                let link = unsafe { &*(*lparam as *const NMLINK) };
                let url = from_wide(&link.item.szUrl);
                let target = if url.is_empty() { from_wide(&link.item.szID) } else { url };
                core.emit(&Event::LinkClicked(target));
                core.emit(&Event::Click);
                Some(0)
            }
            _ => None,
        });
        Ok(Hyperlink { core })
    }
}

impl Hyperlink {
    /// Starts building (default size 200×20).
    pub fn builder() -> HyperlinkBuilder {
        HyperlinkBuilder { opts: ControlOptions::new(200, 20) }
    }

    /// A link was invoked; receives its `href` (or `id` if there is no `href`).
    pub fn on_link_click(&self, f: impl Fn(&str) + 'static) -> HandlerId {
        self.core.on(EventKind::LinkClicked, move |e| {
            if let Event::LinkClicked(target) = e {
                f(target)
            }
        })
    }

    /// Former name of [`on_link_click`](Self::on_link_click), retained for existing code.
    #[deprecated(since = "0.1.0", note = "use `on_link_click`: events follow the naming convention in docs/EVENTS.md")]
    pub fn on_link_clicked(&self, f: impl Fn(&str) + 'static) -> HandlerId {
        self.on_link_click(f)
    }

    /// Opens clicked links in the system's default program.
    pub fn open_links_in_browser(&self) -> HandlerId {
        self.on_link_click(|target| {
            // A falha ao abrir (por exemplo, URL sem programa associado) não tem a quem
            // ser reportada dentro do manipulador; quem precisar tratá-la usa
            // `on_link_clicked` com `system::shell::open` diretamente.
            let _ = crate::system::shell::open(target);
        })
    }
}

// ---- HeaderControl ---------------------------------------------------------------

widget_type!(
    /// Standalone column header (the same control used internally by ListView).
    HeaderControl
);

plain_builder!(
    /// Builder for [`HeaderControl`].
    HeaderControlBuilder
);

fn blank_header_item() -> HDITEMW {
    HDITEMW {
        mask: 0,
        cxy: 0,
        pszText: std::ptr::null_mut(),
        hbm: std::ptr::null_mut(),
        cchTextMax: 0,
        fmt: 0,
        lParam: 0,
        iImage: 0,
        iOrder: 0,
        r#type: 0,
        pvFilter: std::ptr::null_mut(),
        state: 0,
    }
}

impl HeaderControlBuilder {
    /// Creates the header.
    pub fn build(self) -> GuiResult<HeaderControl> {
        let core = self.opts.create("HeaderControl", "SysHeader32", HDS_BUTTONS, 0, false, None)?;
        core.set_reflect(|core, reflected| match reflected {
            Reflected::Notify { code, lparam } if *code == HDN_ITEMCLICKW => {
                // SAFETY: nesta notificação o LPARAM aponta para um NMHEADERW válido.
                let info = unsafe { &*(*lparam as *const NMHEADERW) };
                core.emit(&Event::ColumnClick(info.iItem.max(0) as usize));
                Some(0)
            }
            Reflected::Notify { code, .. } if *code == HDN_ITEMCHANGEDW => {
                core.emit(&Event::ValueChanged);
                None
            }
            _ => None,
        });
        Ok(HeaderControl { core })
    }
}

impl HeaderControl {
    /// Starts building (default size 300×24).
    pub fn builder() -> HeaderControlBuilder {
        HeaderControlBuilder { opts: ControlOptions::new(300, 24) }
    }

    /// Adds an item with the given logical width and returns its index.
    pub fn add_item(&self, text: &str, width: i32) -> usize {
        let mut wide = to_wide(text);
        let mut item = blank_header_item();
        item.mask = HDI_TEXT | HDI_WIDTH | HDI_FORMAT;
        item.cxy = to_pixels(width, self.core.dpi());
        item.pszText = wide.as_mut_ptr();
        item.fmt = HDF_STRING;
        native::send_ref(self.core.hwnd(), HDM_INSERTITEMW, self.item_count(), &item).max(0) as usize
    }

    /// Removes an item.
    pub fn remove_item(&self, index: usize) {
        native::send(self.core.hwnd(), HDM_DELETEITEM, index, 0);
    }

    /// Number of items.
    pub fn item_count(&self) -> usize {
        native::send(self.core.hwnd(), HDM_GETITEMCOUNT, 0, 0).max(0) as usize
    }

    /// Current logical width of an item (users can drag dividers).
    pub fn item_width(&self, index: usize) -> i32 {
        let mut item = blank_header_item();
        item.mask = HDI_WIDTH;
        native::send_mut(self.core.hwnd(), HDM_GETITEMW, index, &mut item);
        to_logical(item.cxy, self.core.dpi())
    }

    /// Shows the sort arrow on an item (`None` removes it).
    pub fn set_sort_indicator(&self, index: usize, ascending: Option<bool>) {
        let mut item = blank_header_item();
        item.mask = HDI_FORMAT;
        item.fmt = HDF_STRING
            | match ascending {
                Some(true) => HDF_SORTUP,
                Some(false) => HDF_SORTDOWN,
                None => 0,
            };
        native::send_ref(self.core.hwnd(), HDM_SETITEMW, index, &item);
    }

    /// An item was clicked; receives its index.
    pub fn on_item_click(&self, f: impl Fn(usize) + 'static) -> HandlerId {
        self.core.on(EventKind::ColumnClick, move |e| {
            if let Event::ColumnClick(index) = e {
                f(*index)
            }
        })
    }

    /// An item's width changed.
    pub fn on_item_resized(&self, f: impl Fn() + 'static) -> HandlerId {
        self.core.on(EventKind::ValueChanged, move |_| f())
    }
}

// ---- Rebar -----------------------------------------------------------------------

widget_type!(
    /// Container of movable bands (Rebar), each hosting a control.
    Rebar
);

impl Rebar {
    /// Creates a Rebar at the top of `parent`.
    pub fn new(parent: &impl Widget) -> GuiResult<Rebar> {
        let core = WidgetCore::create_child(
            "Rebar",
            parent.core(),
            "ReBarWindow32",
            "",
            WS_VISIBLE | WS_CLIPCHILDREN | WS_CLIPSIBLINGS | RBS_VARHEIGHT | RBS_BANDBORDERS | CCS_NODIVIDER,
            0,
            crate::utilities::Rect::default(),
            None,
        )?;
        let raw = core.hwnd() as isize;
        parent.core().on(EventKind::Resize, move |_| {
            native::send(raw as _, WM_SIZE, 0, 0);
        });
        Ok(Rebar { core })
    }

    /// Adds a titled band hosting `child` (which must have been created with this Rebar as its parent).
    pub fn add_band(&self, title: &str, child: &impl Widget, new_row: bool) -> GuiResult<()> {
        let hwnd = self.core.require_alive()?;
        let child_hwnd = child.core().require_alive()?;
        let size = child.size();
        let dpi = self.core.dpi();
        let mut text = to_wide(title);
        let band = REBARBANDINFOW {
            cbSize: std::mem::size_of::<REBARBANDINFOW>() as u32,
            fMask: RBBIM_STYLE | RBBIM_TEXT | RBBIM_CHILD | RBBIM_CHILDSIZE | RBBIM_SIZE,
            fStyle: RBBS_CHILDEDGE | RBBS_GRIPPERALWAYS | if new_row { RBBS_BREAK } else { 0 },
            clrFore: 0,
            clrBack: 0,
            lpText: text.as_mut_ptr(),
            cch: 0,
            iImage: 0,
            hwndChild: child_hwnd,
            cxMinChild: to_pixels(size.width.min(60), dpi) as u32,
            cyMinChild: to_pixels(size.height, dpi) as u32,
            cx: to_pixels(size.width + 80, dpi) as u32,
            hbmBack: std::ptr::null_mut(),
            wID: 0,
            cyChild: 0,
            cyMaxChild: 0,
            cyIntegral: 0,
            cxIdeal: 0,
            lParam: 0,
            cxHeader: 0,
            rcChevronLocation: Default::default(),
            uChevronState: 0,
        };
        if native::send_ref(hwnd, RB_INSERTBANDW, usize::MAX, &band) == 0 {
            return Err(GuiError::InvalidArgument("could not insert the band into the Rebar".into()));
        }
        Ok(())
    }

    /// Removes a band by index.
    pub fn remove_band(&self, index: usize) {
        native::send(self.core.hwnd(), RB_DELETEBAND, index, 0);
    }

    /// Number of bands.
    pub fn band_count(&self) -> usize {
        native::send(self.core.hwnd(), RB_GETBANDCOUNT, 0, 0).max(0) as usize
    }

    /// Logical height occupied by the Rebar.
    pub fn height(&self) -> i32 {
        to_logical(native::send(self.core.hwnd(), RB_GETBARHEIGHT, 0, 0) as i32, self.core.dpi())
    }
}

// ---- Pager -----------------------------------------------------------------------

widget_type!(
    /// Container with scroll buttons for a child larger than the visible area (Pager).
    Pager
);

/// Builder for [`Pager`].
#[derive(Clone, Debug)]
pub struct PagerBuilder {
    opts: ControlOptions,
    horizontal: bool,
}

impl PagerBuilder {
    super::common::builder_common!();

    /// Horizontal scrolling (default: vertical).
    pub fn horizontal(mut self, horizontal: bool) -> Self {
        self.horizontal = horizontal;
        self
    }

    /// Creates the Pager.
    pub fn build(self) -> GuiResult<Pager> {
        let style = if self.horizontal { PGS_HORZ } else { 0 };
        let core = self.opts.create("Pager", "SysPager", style, 0, false, None)?;
        Ok(Pager { core })
    }
}

impl Pager {
    /// Starts building (default size 200×40).
    pub fn builder() -> PagerBuilder {
        PagerBuilder { opts: ControlOptions::new(200, 40), horizontal: false }
    }

    /// Sets the control scrolled by the Pager (must have been created with the Pager as its parent).
    ///
    /// The child's current size is used as the total content size.
    pub fn set_child(&self, child: &impl Widget) -> GuiResult<()> {
        let hwnd = self.core.require_alive()?;
        let child_core = child.core().clone();
        let child_hwnd = child_core.require_alive()?;
        self.core.set_reflect(move |core, reflected| match reflected {
            Reflected::Notify { code, lparam } if *code == PGN_CALCSIZE => {
                // SAFETY: nesta notificação o LPARAM aponta para um NMPGCALCSIZE gravável.
                let calc = unsafe { &mut *(*lparam as *mut NMPGCALCSIZE) };
                let size = child_core.bounds().size();
                let dpi = core.dpi();
                if calc.dwFlag == PGF_CALCWIDTH {
                    calc.iWidth = to_pixels(size.width, dpi);
                } else if calc.dwFlag == PGF_CALCHEIGHT {
                    calc.iHeight = to_pixels(size.height, dpi);
                }
                Some(0)
            }
            _ => None,
        });
        native::send(hwnd, PGM_SETCHILD, 0, child_hwnd as isize);
        native::send(hwnd, PGM_RECALCSIZE, 0, 0);
        Ok(())
    }

    /// Current scroll position in logical units.
    pub fn scroll_position(&self) -> i32 {
        to_logical(native::send(self.core.hwnd(), PGM_GETPOS, 0, 0) as i32, self.core.dpi())
    }

    /// Sets the scroll position.
    pub fn set_scroll_position(&self, position: i32) {
        native::send(self.core.hwnd(), PGM_SETPOS, 0, to_pixels(position, self.core.dpi()) as isize);
    }
}

// ---- HotKeyBox -------------------------------------------------------------------

widget_type!(
    /// Field capturing a key combination typed by the user (HotKey).
    HotKeyBox
);

plain_builder!(
    /// Builder for [`HotKeyBox`].
    HotKeyBoxBuilder
);

impl HotKeyBoxBuilder {
    /// Creates the field.
    pub fn build(self) -> GuiResult<HotKeyBox> {
        let core = self.opts.create("HotKeyBox", "msctls_hotkey32", 0, WS_EX_CLIENTEDGE, true, None)?;
        core.set_centered_text(true);
        core.set_reflect(|core, reflected| match reflected {
            Reflected::Command { code, .. } if *code == EN_CHANGE => {
                core.emit(&Event::ValueChanged);
                Some(0)
            }
            _ => None,
        });
        Ok(HotKeyBox { core })
    }
}

impl HotKeyBox {
    /// Starts building (default size 160 × [`DEFAULT_CONTROL_HEIGHT`](super::DEFAULT_CONTROL_HEIGHT)).
    pub fn builder() -> HotKeyBoxBuilder {
        HotKeyBoxBuilder { opts: ControlOptions::new(160, super::DEFAULT_CONTROL_HEIGHT) }
    }

    /// Current combination, if any.
    pub fn shortcut(&self) -> Option<Shortcut> {
        let value = native::send(self.core.hwnd(), HKM_GETHOTKEY, 0, 0) as u16;
        let (key, flags) = ((value & 0xFF), (value >> 8) as u8);
        (key != 0).then_some(Shortcut {
            key: Key(key),
            modifiers: Modifiers {
                ctrl: flags & HOTKEYF_CONTROL != 0,
                shift: flags & HOTKEYF_SHIFT != 0,
                alt: flags & HOTKEYF_ALT != 0,
                win: false,
            },
        })
    }

    /// Sets the displayed combination (`None` clears the field).
    pub fn set_shortcut(&self, shortcut: Option<Shortcut>) {
        let value = shortcut
            .map(|s| {
                let mut flags = 0u8;
                if s.modifiers.ctrl {
                    flags |= HOTKEYF_CONTROL;
                }
                if s.modifiers.shift {
                    flags |= HOTKEYF_SHIFT;
                }
                if s.modifiers.alt {
                    flags |= HOTKEYF_ALT;
                }
                ((flags as usize) << 8) | (s.key.0 as usize & 0xFF)
            })
            .unwrap_or(0);
        native::send(self.core.hwnd(), HKM_SETHOTKEY, value, 0);
    }

    /// The user typed another combination.
    pub fn on_value_changed(&self, f: impl Fn(Option<Shortcut>) + 'static) -> HandlerId {
        let this = self.clone();
        self.core.on(EventKind::ValueChanged, move |_| f(this.shortcut()))
    }
}

// ---- IpAddressBox ----------------------------------------------------------------

widget_type!(
    /// IPv4 address field with four validated octets.
    IpAddressBox
);

plain_builder!(
    /// Builder for [`IpAddressBox`].
    IpAddressBoxBuilder
);

impl IpAddressBoxBuilder {
    /// Creates the field.
    pub fn build(self) -> GuiResult<IpAddressBox> {
        let core = self.opts.create("IpAddressBox", "SysIPAddress32", 0, WS_EX_CLIENTEDGE, true, None)?;
        core.set_reflect(|core, reflected| match reflected {
            Reflected::Notify { code, .. } if *code == IPN_FIELDCHANGED => {
                core.emit(&Event::ValueChanged);
                Some(0)
            }
            Reflected::Command { code, .. } if *code == EN_CHANGE => {
                core.emit(&Event::ValueChanged);
                Some(0)
            }
            _ => None,
        });
        Ok(IpAddressBox { core })
    }
}

impl IpAddressBox {
    /// Starts building (default size 150×24). This is the only single-line field that does not use
    /// [`DEFAULT_CONTROL_HEIGHT`](super::DEFAULT_CONTROL_HEIGHT): the native control positions its four internal fields
    /// according to its height and does not support centering them.
    pub fn builder() -> IpAddressBoxBuilder {
        IpAddressBoxBuilder { opts: ControlOptions::new(150, 24) }
    }

    /// Entered address; `None` while any octet is empty.
    pub fn address(&self) -> Option<Ipv4Addr> {
        let mut packed = 0u32;
        let filled = native::send_mut(self.core.hwnd(), IPM_GETADDRESS, 0, &mut packed);
        (filled == 4).then(|| Ipv4Addr::from(packed))
    }

    /// Sets the displayed address.
    pub fn set_address(&self, address: Ipv4Addr) {
        native::send(self.core.hwnd(), IPM_SETADDRESS, 0, u32::from(address) as isize);
    }

    /// Clears all octets.
    pub fn clear(&self) {
        native::send(self.core.hwnd(), IPM_CLEARADDRESS, 0, 0);
    }

    /// The user changed the address.
    pub fn on_value_changed(&self, f: impl Fn(Option<Ipv4Addr>) + 'static) -> HandlerId {
        let this = self.clone();
        self.core.on(EventKind::ValueChanged, move |_| f(this.address()))
    }
}

// ---- Animation -------------------------------------------------------------------

widget_type!(
    /// Simple silent AVI animation player (Animation control).
    Animation
);

plain_builder!(
    /// Builder for [`Animation`].
    AnimationBuilder
);

impl AnimationBuilder {
    /// Creates the control.
    pub fn build(self) -> GuiResult<Animation> {
        let core = self.opts.create("Animation", "SysAnimate32", ACS_CENTER | ACS_TRANSPARENT, 0, false, None)?;
        Ok(Animation { core })
    }
}

impl Animation {
    /// Starts building (default size 120×60).
    pub fn builder() -> AnimationBuilder {
        AnimationBuilder { opts: ControlOptions::new(120, 60) }
    }

    /// Opens an `.avi` file (uncompressed or RLE, without audio).
    pub fn open(&self, path: impl AsRef<Path>) -> GuiResult<()> {
        let hwnd = self.core.require_alive()?;
        if native::send_text(hwnd, ACM_OPENW, 0, &path.as_ref().to_string_lossy()) == 0 {
            return Err(GuiError::InvalidArgument(format!(
                "the Animation control could not open {}",
                path.as_ref().display()
            )));
        }
        Ok(())
    }

    /// Plays the animation `repetitions` times (`None` = indefinitely).
    pub fn play(&self, repetitions: Option<u32>) {
        // LPARAM: quadro inicial 0 na palavra baixa e quadro final -1 (último) na alta.
        let frames = crate::ffi::make_lparam(0, 0xFFFF);
        native::send(self.core.hwnd(), ACM_PLAY, repetitions.map(|r| r as usize).unwrap_or(usize::MAX), frames);
    }

    /// Stops playback.
    pub fn stop(&self) {
        native::send(self.core.hwnd(), ACM_STOP, 0, 0);
    }
}
