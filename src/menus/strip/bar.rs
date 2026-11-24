//! [`MenuStrip`](super::MenuStrip) `StripInner` state/behavior: measurement, overflow, drawing, nonclient area,
//! keyboard and highlights.

use super::*;

impl StripInner {
    pub(super) fn bump(&self) {
        self.generation.set(self.generation.get() + 1);
        self.layout.borrow_mut().take();
        self.repaint();
    }

    pub(crate) fn owner(&self) -> HWND {
        self.owner.get()
    }

    /// Effective explicit/theme/system style.
    pub(crate) fn effective_style(&self) -> MenuStyle {
        if let Some(style) = self.style.get() {
            return style;
        }
        style_for_window(self.owner.get())
    }

    pub(crate) fn font(&self) -> Font {
        font_for_window(self.owner.get())
    }

    pub(crate) fn dpi(&self) -> u32 {
        native::dpi_for_window(self.owner.get())
    }

    /// Logical strip height.
    pub(crate) fn height(&self) -> i32 {
        bar_height(&self.font(), self.dpi())
    }

    pub(super) fn height_px(&self) -> i32 {
        let dpi = self.dpi();
        to_pixels(bar_height(&self.font(), dpi), dpi)
    }

    /// Strip screen pixel rectangle if visible and not minimized.
    pub(crate) fn band_screen_rect(&self) -> Option<RECT> {
        let hwnd = self.owner.get();
        // SAFETY: consulta simples.
        if hwnd.is_null() || unsafe { IsIconic(hwnd) } != 0 {
            return None;
        }
        let mut origin = POINT { x: 0, y: 0 };
        // SAFETY: ponteiro válido para um POINT local.
        unsafe { ClientToScreen(hwnd, &mut origin) };
        let client = native::client_rect(hwnd);
        let height = self.height_px();
        Some(RECT { left: origin.x, top: origin.y - height, right: origin.x + client.right, bottom: origin.y })
    }

    /// Visible positions for current width.
    pub(crate) fn slots(&self) -> Vec<Slot> {
        let Some(band) = self.band_screen_rect() else { return Vec::new() };
        let dpi = self.dpi();
        let key = LayoutKey {
            width: to_logical(band.right - band.left, dpi),
            dpi,
            font: self.font(),
            generation: self.generation.get(),
        };
        if let Some((cached, slots)) = self.layout.borrow().as_ref() {
            if *cached == key {
                return slots.clone();
            }
        }
        let height = bar_height(&key.font, dpi);
        let widths: Vec<i32> = {
            let items = self.items.borrow();
            with_measure(dpi, |canvas| {
                items
                    .iter()
                    .map(|item| {
                        let text = canvas.measure_mnemonic(&item.text, &key.font).width;
                        let icon = if item.icon.is_some() { ICON + BAR_ICON_GAP } else { 0 };
                        text + icon + 2 * BAR_PAD_X
                    })
                    .collect()
            })
            .unwrap_or_default()
        };
        let slots = layout_slots(&widths, key.width, height);
        *self.layout.borrow_mut() = Some((key, slots.clone()));
        slots
    }

    /// Position under screen point.
    pub(crate) fn slot_at_screen(&self, point: POINT) -> Option<usize> {
        let band = self.band_screen_rect()?;
        if point.x < band.left || point.x >= band.right || point.y < band.top || point.y >= band.bottom {
            return None;
        }
        let dpi = self.dpi();
        let local = Point::new(to_logical(point.x - band.left, dpi), to_logical(point.y - band.top, dpi));
        self.slots().iter().position(|slot| slot.rect.contains(local))
    }

    pub(crate) fn band_contains(&self, point: POINT) -> bool {
        self.band_screen_rect()
            .is_some_and(|b| point.x >= b.left && point.x < b.right && point.y >= b.top && point.y < b.bottom)
    }

    /// Position screen pixel rectangle.
    pub(crate) fn slot_screen_rect(&self, slot: usize) -> Option<RECT> {
        let band = self.band_screen_rect()?;
        let rect = self.slots().get(slot)?.rect;
        let dpi = self.dpi();
        let left = band.left + to_pixels(rect.x, dpi);
        Some(RECT { left, top: band.top, right: left + to_pixels(rect.width, dpi), bottom: band.bottom })
    }

    pub(crate) fn slot_count(&self) -> usize {
        self.slots().len()
    }

    fn slot_enabled(&self, slot: &Slot) -> bool {
        match slot.kind {
            SlotKind::Item(index) => self.items.borrow().get(index).is_some_and(|i| i.enabled),
            SlotKind::Overflow => true,
        }
    }

    /// Menu to open, `None` when disabled.
    pub(crate) fn slot_menu(&self, slot: usize) -> Option<SlotMenu> {
        let slots = self.slots();
        let target = slots.get(slot)?;
        match target.kind {
            SlotKind::Item(index) => {
                let items = self.items.borrow();
                let item = items.get(index)?;
                item.enabled.then(|| SlotMenu::Menu(item.menu.inner.clone()))
            }
            SlotKind::Overflow => {
                let visible: Vec<usize> = slots
                    .iter()
                    .filter_map(|s| match s.kind {
                        SlotKind::Item(i) => Some(i),
                        SlotKind::Overflow => None,
                    })
                    .collect();
                let items = self.items.borrow();
                let entries = items
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !visible.contains(i))
                    .map(|(_, item)| Entry {
                        kind: EntryKind::Submenu(item.menu.inner.clone()),
                        text: item.text.clone(),
                        shortcut: String::new(),
                        enabled: item.enabled,
                        checked: false,
                        radio: false,
                        default: false,
                        icon: item.icon.clone(),
                    })
                    .collect();
                Some(SlotMenu::Overflow(entries))
            }
        }
    }

    /// Next position circularly from `from`.
    pub(crate) fn next_slot(&self, from: Option<usize>, forward: bool) -> Option<usize> {
        let count = self.slot_count();
        if count == 0 {
            return None;
        }
        Some(match from {
            None => {
                if forward {
                    0
                } else {
                    count - 1
                }
            }
            Some(current) if forward => (current + 1) % count,
            Some(current) => (current + count - 1) % count,
        })
    }

    /// Enabled visible position matching mnemonic `c`.
    pub(crate) fn slot_by_mnemonic(&self, c: char) -> Option<usize> {
        let c = c.to_lowercase().next()?;
        let items = self.items.borrow();
        self.slots().iter().position(|slot| match slot.kind {
            SlotKind::Item(index) => {
                items.get(index).is_some_and(|item| item.enabled && mnemonic_of(&item.text) == Some(c))
            }
            SlotKind::Overflow => false,
        })
    }

    pub(crate) fn set_state(&self, hot: Option<usize>, open: Option<usize>) {
        if self.hot.get() != hot || self.open.get() != open {
            self.hot.set(hot);
            self.open.set(open);
            self.repaint();
            // Seleção pelo teclado na própria faixa (sem lista aberta): avisa o foco.
            if let (Some(slot), None) = (hot, open) {
                if popup::current_owner() == Some(self.owner()) {
                    let host = lookup(self.owner()).and_then(|core| core.accessible_tree.borrow().clone());
                    if let Some(host) = host {
                        host.focus_changed(STRIP_ITEM_BASE + slot as u32);
                    }
                }
            }
        }
    }

    /// Position accessible name.
    pub(crate) fn slot_name(&self, slot: usize) -> Option<String> {
        match self.slots().get(slot)?.kind {
            SlotKind::Item(index) => self.items.borrow().get(index).map(|item| plain_text(&item.text)),
            SlotKind::Overflow => Some(crate::i18n::library_text(crate::i18n::LibraryText::MoreMenus)),
        }
    }

    pub(crate) fn hot(&self) -> Option<usize> {
        self.hot.get()
    }

    pub(crate) fn open_slot(&self) -> Option<usize> {
        self.open.get()
    }

    /// Returns to idle after menu mode.
    pub(crate) fn reset_after_tracking(&self) {
        self.alt_cues.set(false);
        let mut cursor = POINT { x: 0, y: 0 };
        // SAFETY: ponteiro válido para um POINT local.
        unsafe { GetCursorPos(&mut cursor) };
        let hot = self.slot_at_screen(cursor);
        self.hot.set(hot);
        self.open.set(None);
        self.repaint();
    }

    /// Executes item matching `shortcut` in any bar menu.
    pub(super) fn try_shortcut(&self, shortcut: Shortcut) -> bool {
        let menus: Vec<(Rc<MenuInner>, bool)> =
            self.items.borrow().iter().map(|item| (item.menu.inner.clone(), item.enabled)).collect();
        for (menu, enabled) in menus {
            if let Some((owner, id)) = menu.find_shortcut(shortcut) {
                owner.notify_open();
                if enabled && owner.is_enabled(id) {
                    app::invoke_menu(id);
                }
                return true;
            }
        }
        false
    }

    /// Repaints strip immediately.
    pub(crate) fn repaint(&self) {
        let hwnd = self.owner.get();
        if hwnd.is_null() {
            return;
        }
        // SAFETY: DC da janela inteira, liberado logo após o desenho.
        unsafe {
            let hdc = GetWindowDC(hwnd);
            if hdc.is_null() {
                return;
            }
            self.paint(hdc);
            ReleaseDC(hwnd, hdc);
        }
    }

    /// Draws strip using whole-window DC.
    fn paint(&self, window_dc: HDC) {
        let hwnd = self.owner.get();
        let Some(band) = self.band_screen_rect() else { return };
        let outer = native::window_rect(hwnd);
        let dpi = self.dpi();
        let style = self.effective_style();
        let font = self.font();
        let slots = self.slots();
        let cues = self.alt_cues.get() || popup::cues_for(hwnd) || system_keyboard_cues();
        let (hot, open) = (self.hot.get(), self.open.get());
        let items = self.items.borrow();
        crate::graphics::gdi::paint_buffered_at(
            window_dc,
            band.left - outer.left,
            band.top - outer.top,
            band.right - band.left,
            band.bottom - band.top,
            dpi,
            style.bar_background,
            |canvas| {
                for (index, slot) in slots.iter().enumerate() {
                    let enabled = self.slot_enabled(slot);
                    let state_open = open == Some(index);
                    let active = state_open || hot == Some(index);
                    let r = slot.rect;
                    if active {
                        let fill = if state_open { style.bar_open } else { style.bar_hover };
                        let back = Rect::new(r.x + 1, r.y + 2, r.width - 2, r.height - 4);
                        canvas.round_rect(back, style.corner_radius, None, Some(fill));
                    }
                    let color = if !enabled {
                        style.disabled_foreground
                    } else if active {
                        style.bar_hover_foreground
                    } else {
                        style.bar_foreground
                    };
                    match slot.kind {
                        SlotKind::Overflow => {
                            canvas.mnemonic_text("»", r, &font, color, TextAlign::Center, false);
                        }
                        SlotKind::Item(i) => {
                            let Some(item) = items.get(i) else { continue };
                            let mut x = r.x + BAR_PAD_X;
                            if let Some(icon) = &item.icon {
                                popup::draw_icon(canvas, icon.as_ref(), x, r.y + (r.height - ICON) / 2, enabled);
                                x += ICON + BAR_ICON_GAP;
                            }
                            let text = Rect::new(x, r.y, r.right() - BAR_PAD_X - x + 2, r.height);
                            canvas.mnemonic_text(&item.text, text, &font, color, TextAlign::Left, cues);
                        }
                    }
                }
            },
        );
    }

    /// Toggles underlines while Alt is down.
    pub(super) fn set_alt_cues(&self, on: bool) {
        if self.alt_cues.get() != on {
            self.alt_cues.set(on);
            self.repaint();
        }
    }

    /// Enters keyboard menu mode (Alt/F10), first menu selected.
    pub(super) fn enter_keyboard_mode(self: &Rc<Self>) {
        if self.slot_count() > 0 {
            popup::enter_bar_keyboard_mode(self.clone());
        }
    }
}
