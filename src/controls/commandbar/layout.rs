//! [`CommandBar`](super::CommandBar) item measurement and positioning: widths, fitting/overflow into `»` and hit
//! testing.

use super::*;

impl BarState {
    pub(super) fn core(&self) -> Option<Rc<WidgetCore>> {
        self.core.upgrade().filter(|c| c.is_alive())
    }

    pub(super) fn hwnd(&self) -> HWND {
        self.core().map_or(std::ptr::null_mut(), |c| c.hwnd())
    }

    pub(super) fn invalidate(&self) {
        if let Some(core) = self.core() {
            core.invalidate();
        }
    }

    pub(super) fn style(&self) -> MenuStyle {
        self.style.get().unwrap_or_else(|| menu_strip::style_for_window(self.hwnd()))
    }

    pub(super) fn font(&self) -> Font {
        self.core().map(|c| c.font()).unwrap_or_else(Font::system)
    }

    fn size(&self) -> (i32, i32, u32) {
        let Some(core) = self.core() else { return (0, 0, 96) };
        let dpi = core.dpi();
        let rect = native::client_rect(core.hwnd());
        (to_logical(rect.right, dpi), to_logical(rect.bottom, dpi), dpi)
    }

    fn button_width(&self, data: &CommandButton, canvas: &Canvas, font: &Font) -> i32 {
        let text =
            if data.show_text && !data.text.is_empty() { canvas.measure_mnemonic(&data.text, font).width } else { 0 };
        let icon = if data.icon.is_some() { ICON } else { 0 };
        let between = if icon > 0 && text > 0 { ICON_TEXT_GAP } else { 0 };
        let mut width = 2 * BUTTON_PAD + icon + between + text;
        if text == 0 {
            width = width.max(ITEM_HEIGHT + 4);
        }
        width
            + match data.mode {
                Mode::Dropdown(_) => ARROW,
                Mode::Split(_) => SPLIT_ARROW,
                _ => 0,
            }
    }

    /// Recalculates positions, repositions hosted controls and updates tooltips.
    pub(super) fn relayout(&self) {
        let Some(core) = self.core() else { return };
        let (width, height, dpi) = self.size();
        let font = self.font();
        let slots: Vec<(u32, Slot)> = {
            let items = self.items.borrow();
            menu_strip::with_measure(dpi, |canvas| {
                items
                    .iter()
                    .filter(|item| item.visible)
                    .map(|item| {
                        let slot = match &item.kind {
                            Kind::Button { data, .. } => Slot::Fixed(self.button_width(data, canvas, &font)),
                            Kind::Separator => Slot::Separator,
                            Kind::Spacer => Slot::Spacer,
                            Kind::Label(text) => {
                                Slot::Fixed(canvas.measure_mnemonic(text, &font).width + 2 * LABEL_PAD)
                            }
                            Kind::Widget(h) | Kind::Search(h) => Slot::Fixed(h.width),
                        };
                        (item.id, slot)
                    })
                    .collect()
            })
            .unwrap_or_default()
        };
        let placement = place(&slots.iter().map(|(_, s)| *s).collect::<Vec<_>>(), width);
        let top = (height - ITEM_HEIGHT) / 2;
        let mut hosted: Vec<(Rc<WidgetCore>, Option<Rect>)> = Vec::new();
        {
            let mut items = self.items.borrow_mut();
            for item in items.iter_mut() {
                item.rect = None;
            }
            for ((id, _), position) in slots.iter().zip(&placement.items) {
                let Some(item) = items.iter_mut().find(|i| i.id == *id) else { continue };
                item.rect = position.map(|(x, w)| Rect::new(x, top, w, ITEM_HEIGHT));
            }
            for item in items.iter() {
                match &item.kind {
                    Kind::Widget(h) => {
                        let rect = item.rect.filter(|_| item.visible).map(|r| {
                            let own = h.core.bounds().height.clamp(1, height);
                            Rect::new(r.x, (height - own) / 2, r.width, own)
                        });
                        hosted.push((h.core.clone(), rect));
                    }
                    Kind::Search(h) => {
                        let rect = item.rect.filter(|_| item.visible).map(|r| {
                            let text = menu_strip::text_height(&font, dpi) + 2;
                            let x = r.x + SEARCH_ICON;
                            let right = r.right() - SEARCH_CLEAR;
                            Rect::new(x, r.y + (r.height - text) / 2, (right - x).max(1), text)
                        });
                        hosted.push((h.core.clone(), rect));
                    }
                    _ => {}
                }
            }
        }
        self.overflow.set(placement.overflow.map(|x| Rect::new(x, top, OVERFLOW, ITEM_HEIGHT)));
        // Fora de qualquer empréstimo: mover e mostrar janelas envia mensagens.
        for (child, rect) in hosted {
            let hwnd = child.hwnd();
            match rect {
                Some(rect) => {
                    child.set_bounds(rect);
                    // SAFETY: janela filha viva desta thread.
                    unsafe { ShowWindow(hwnd, SW_SHOWNA) };
                }
                // SAFETY: janela filha viva desta thread.
                None => unsafe {
                    ShowWindow(hwnd, SW_HIDE);
                },
            }
        }
        self.update_tooltips(&core, dpi);
        core.invalidate();
    }

    fn update_tooltips(&self, core: &Rc<WidgetCore>, dpi: u32) {
        let to_native = |r: Rect| RECT {
            left: to_pixels(r.x, dpi),
            top: to_pixels(r.y, dpi),
            right: to_pixels(r.right(), dpi),
            bottom: to_pixels(r.bottom(), dpi),
        };
        let mut regions: Vec<(usize, RECT, String)> = self
            .items
            .borrow()
            .iter()
            .filter_map(|item| match (&item.kind, item.rect) {
                (Kind::Button { data, .. }, Some(rect)) => {
                    Some((item.id as usize, to_native(rect), data.tooltip_text()))
                }
                _ => None,
            })
            .collect();
        if let Some(rect) = self.overflow.get() {
            regions.push((
                OVERFLOW_ID as usize,
                to_native(rect),
                crate::i18n::library_text(crate::i18n::LibraryText::MoreCommands),
            ));
        }
        let previous = self.tooltip_ids.replace(regions.iter().map(|(id, _, _)| *id).collect());
        let _ = crate::controls::tooltip::set_regions(core, &previous, &regions);
    }

    /// Item and section at a logical bar point.
    pub(super) fn hit(&self, point: Point) -> Option<(u32, Part)> {
        if self.overflow.get().is_some_and(|r| r.contains(point)) {
            return Some((OVERFLOW_ID, Part::Main));
        }
        let items = self.items.borrow();
        let item = items.iter().find(|i| i.rect.is_some_and(|r| r.contains(point)))?;
        let rect = item.rect?;
        let part = match &item.kind {
            Kind::Button { data, .. } => match data.mode {
                Mode::Split(_) if point.x >= rect.right() - SPLIT_ARROW => Part::Arrow,
                _ => Part::Main,
            },
            Kind::Search(h) => {
                let has_text = !native::window_text(h.core.hwnd()).is_empty();
                if has_text && point.x >= rect.right() - SEARCH_CLEAR {
                    Part::Clear
                } else {
                    Part::Main
                }
            }
            Kind::Separator | Kind::Spacer | Kind::Label(_) | Kind::Widget(_) => return None,
        };
        Some((item.id, part))
    }

    /// Item under the current cursor position.
    pub(super) fn hit_at_cursor(&self) -> Option<(u32, Part)> {
        let core = self.core()?;
        let mut point = POINT { x: 0, y: 0 };
        // SAFETY: ponteiros válidos para um POINT local.
        unsafe {
            GetCursorPos(&mut point);
            ScreenToClient(core.hwnd(), &mut point);
        }
        let dpi = core.dpi();
        self.hit(Point::new(to_logical(point.x, dpi), to_logical(point.y, dpi)))
    }

    pub(super) fn screen_rect(&self, id: u32) -> Option<RECT> {
        let core = self.core()?;
        let rect = if id == OVERFLOW_ID {
            self.overflow.get()?
        } else {
            self.items.borrow().iter().find(|i| i.id == id)?.rect?
        };
        let dpi = core.dpi();
        let mut origin = POINT { x: 0, y: 0 };
        // SAFETY: ponteiro válido para um POINT local.
        unsafe { ClientToScreen(core.hwnd(), &mut origin) };
        let left = origin.x + to_pixels(rect.x, dpi);
        let top = origin.y + to_pixels(rect.y, dpi);
        Some(RECT { left, top, right: left + to_pixels(rect.width, dpi), bottom: top + to_pixels(rect.height, dpi) })
    }

    /// Insertion position (`items` index) for a logical x between displayed items.
    pub(super) fn insertion_at(&self, x: i32) -> Option<usize> {
        let items = self.items.borrow();
        let shown: Vec<(usize, Rect)> = items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| item.rect.filter(|_| item.visible).map(|r| (i, r)))
            .collect();
        let (_, last) = shown.last().copied()?;
        for (index, rect) in &shown {
            if x < rect.x + rect.width / 2 {
                return Some(*index);
            }
        }
        let _ = last;
        Some(shown.last().map(|(i, _)| i + 1).unwrap_or(items.len()))
    }

    /// Logical x of the insertion line before item `index` or after the last item.
    pub(super) fn insertion_x(&self, index: usize) -> i32 {
        let items = self.items.borrow();
        if let Some(rect) = items.get(index).and_then(|i| i.rect) {
            return rect.x - GAP / 2 - 1;
        }
        items.iter().filter_map(|i| i.rect).map(|r| r.right() + GAP / 2).max().unwrap_or(PAD_X)
    }

    /// Moves item `id` before `items` position `target`; notifies `on_reordered`.
    pub(super) fn move_item(&self, id: u32, target: usize) -> bool {
        let moved = {
            let mut items = self.items.borrow_mut();
            let Some(from) = items.iter().position(|i| i.id == id) else { return false };
            if target == from || target == from + 1 {
                false
            } else {
                let item = items.remove(from);
                let to = if target > from { target - 1 } else { target };
                let to = to.min(items.len());
                items.insert(to, item);
                true
            }
        };
        if moved {
            self.relayout();
            let listeners = self.reordered.borrow().clone();
            for listener in listeners {
                listener();
            }
        }
        moved
    }

    pub(super) fn button(&self, id: u32) -> Option<(CommandButton, u32)> {
        self.items.borrow().iter().find(|i| i.id == id).and_then(|item| match &item.kind {
            Kind::Button { data, menu_id } => Some((data.clone(), *menu_id)),
            _ => None,
        })
    }

    /// Items in `»` as menu entries.
    pub(super) fn overflow_entries(&self) -> Vec<Entry> {
        let mut entries: Vec<Entry> = Vec::new();
        for item in self.items.borrow().iter().filter(|i| i.visible && i.rect.is_none()) {
            match &item.kind {
                Kind::Button { data, menu_id } => {
                    let kind = match data.menu_of() {
                        Some(menu) => EntryKind::Submenu(menu.inner.clone()),
                        None => EntryKind::Command(*menu_id),
                    };
                    entries.push(Entry {
                        kind,
                        text: data.text.clone(),
                        shortcut: String::new(),
                        enabled: data.enabled,
                        checked: matches!(data.mode, Mode::Toggle) && data.checked,
                        radio: false,
                        default: false,
                        icon: data.icon.clone(),
                    });
                }
                Kind::Widget(h) | Kind::Search(h) => entries.push(Entry {
                    kind: EntryKind::Command(h.menu_id),
                    text: format!("{}…", h.name),
                    shortcut: String::new(),
                    enabled: native::is_enabled(h.core.hwnd()),
                    checked: false,
                    radio: false,
                    default: false,
                    icon: None,
                }),
                Kind::Separator if entries.last().is_some_and(|e| !e.is_separator()) => entries.push(Entry {
                    kind: EntryKind::Separator,
                    text: String::new(),
                    shortcut: String::new(),
                    enabled: true,
                    checked: false,
                    radio: false,
                    default: false,
                    icon: None,
                }),
                _ => {}
            }
        }
        if entries.last().is_some_and(Entry::is_separator) {
            entries.pop();
        }
        entries
    }
}
