//! Drawn menu state machine ([`Tracker`](super::Tracker)): levels, keyboard, mouse, scrolling, submenu timers and
//! selection.

use super::*;

impl Tracker {
    pub(super) fn level(&self, index: usize) -> Option<Rc<Level>> {
        self.levels.borrow().get(index).cloned()
    }

    pub(crate) fn owner(&self) -> HWND {
        self.owner
    }

    pub(crate) fn depth(&self) -> usize {
        self.levels.borrow().len()
    }

    fn deepest(&self) -> Option<(usize, Rc<Level>)> {
        let levels = self.levels.borrow();
        levels.last().map(|l| (levels.len() - 1, l.clone()))
    }

    /// Closes lists from level `from` inclusive.
    pub(super) fn close_from(&self, from: usize) {
        let closing: Vec<Rc<Level>> = {
            let mut levels = self.levels.borrow_mut();
            if from >= levels.len() {
                return;
            }
            levels.split_off(from)
        };
        if self.pending.get().is_some_and(|(level, _)| level + 1 >= from) {
            self.pending.set(None);
        }
        for level in closing.into_iter().rev() {
            level.destroy();
        }
    }

    pub(super) fn open_slot(&self, slot: usize, keyboard: bool) {
        let Some(strip) = self.strip.clone() else { return };
        self.close_from(0);
        strip.set_state(Some(slot), Some(slot));
        let Some(source) = strip.slot_menu(slot) else { return };
        let (menu, entries) = match source {
            SlotMenu::Menu(menu) => {
                menu.notify_open();
                let entries = menu.entries();
                (Some(menu), entries)
            }
            SlotMenu::Overflow(entries) => (None, entries),
        };
        if entries.is_empty() {
            return;
        }
        let Some(anchor) = strip.slot_screen_rect(slot) else { return };
        let min_width = to_logical(anchor.right - anchor.left, self.dpi);
        self.push_level(menu, entries, Placement::Below(anchor), None, keyboard, min_width);
    }

    /// Opens `item` submenu at `level`.
    pub(super) fn open_child(&self, level: usize, item: usize, keyboard: bool) {
        let Some(parent) = self.level(level) else { return };
        let EntryKind::Submenu(menu) = &parent.entries[item].kind else { return };
        if !parent.entries[item].enabled {
            return;
        }
        if let Some(child) = self.level(level + 1) {
            if child.child_of == Some(item) {
                if keyboard && child.hot.get().is_none() {
                    child.set_hot(child.step(true));
                }
                return;
            }
        }
        self.close_from(level + 1);
        parent.set_hot(Some(item));
        let menu = menu.clone();
        menu.notify_open();
        let entries = menu.entries();
        if entries.is_empty() {
            return;
        }
        let placement = Placement::Beside { parent: parent.screen.get(), row: parent.row_screen_rect(item) };
        self.push_level(Some(menu), entries, placement, Some(item), keyboard, 0);
    }

    pub(super) fn push_level(
        &self,
        menu: Option<Rc<MenuInner>>,
        entries: Vec<Entry>,
        placement: Placement,
        child_of: Option<usize>,
        keyboard: bool,
        min_width: i32,
    ) {
        let dpi = self.dpi;
        let layout = measure(&entries, &self.font, dpi, min_width);
        let (width, height) = (to_pixels(layout.width, dpi), to_pixels(layout.height, dpi));
        let anchor = match &placement {
            Placement::Below(r) => POINT { x: r.left, y: r.bottom },
            Placement::Beside { row, .. } => POINT { x: row.right, y: row.top },
            Placement::At(p) => *p,
        };
        let work = Monitor::from_point(Point::new(anchor.x, anchor.y))
            .map(|m| m.work_area.to_native())
            .unwrap_or(RECT { left: 0, top: 0, right: 1 << 15, bottom: 1 << 15 });
        let rect = place(&placement, width, height, work, to_pixels(PAD_Y + BORDER, dpi), to_pixels(2, dpi));
        let level = Rc::new(Level {
            hwnd: Cell::new(std::ptr::null_mut()),
            entries,
            layout,
            view_height: Cell::new(to_logical(rect.bottom - rect.top, dpi)),
            scroll: Cell::new(0),
            hot: Cell::new(None),
            child_of,
            screen: Cell::new(rect),
            tracking_leave: Cell::new(false),
            dpi,
            style: self.style,
            font: self.font.clone(),
            menu,
            accessible: RefCell::new(None),
        });
        if keyboard {
            level.hot.set(level.step(true));
            level.announce_highlight();
        }
        let Some(hwnd) = create_popup(self.owner, rect, &self.style) else { return };
        level.hwnd.set(hwnd);
        POPUPS.with(|p| p.borrow_mut().insert(hwnd as isize, level.clone()));
        self.levels.borrow_mut().push(level.clone());
        // SAFETY: mostra a popup recém-criada sem ativá-la, acima da janela dona, e avisa
        // as tecnologias assistivas (MSAA) de que uma lista de menu abriu.
        unsafe {
            SetWindowPos(hwnd, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
            NotifyWinEvent(EVENT_SYSTEM_MENUPOPUPSTART, hwnd, OBJID_CLIENT, 0);
        }
        // Com um leitor de tela ativo, publica os elementos já e anuncia o item em foco.
        if clients_listening() {
            if let Some(host) = level.accessible_host() {
                host.raise(0, crate::ffi::uiautomation::UIA_MENU_OPENED_EVENT_ID);
                if let Some(index) = level.hot.get() {
                    host.focus_changed(index as u32 + 1);
                }
            }
        }
    }

    /// Executes `item` at `level` or opens submenu.
    pub(super) fn activate(&self, level: usize, item: usize) {
        let Some(list) = self.level(level) else { return };
        let entry = &list.entries[item];
        if !entry.enabled {
            return;
        }
        match &entry.kind {
            EntryKind::Command(id) => {
                let id = *id;
                end();
                app::invoke_menu(id);
            }
            EntryKind::Submenu(_) => self.open_child(level, item, true),
            EntryKind::Separator => {}
        }
    }

    fn schedule(&self, level: &Level, index: usize, action: Option<usize>) {
        self.pending.set(Some((index, action)));
        let mut delay: u32 = 400;
        // SAFETY: ponteiro de saída válido para um u32.
        unsafe { SystemParametersInfoW(SPI_GETMENUSHOWDELAY, 0, &mut delay as *mut u32 as *mut _, 0) };
        // SAFETY: temporizador da própria popup; WM_TIMER chega a `popup_proc`.
        unsafe { SetTimer(level.hwnd.get(), OPEN_TIMER, delay.max(1), std::ptr::null()) };
    }

    pub(super) fn on_timer(&self, hwnd: HWND) {
        let index = self.levels.borrow().iter().position(|l| l.hwnd.get() == hwnd);
        let Some(index) = index else { return };
        let Some((level, action)) = self.pending.get() else { return };
        if level != index {
            return;
        }
        self.pending.set(None);
        let Some(list) = self.level(index) else { return };
        match action {
            Some(item) if list.hot.get() == Some(item) => self.open_child(index, item, false),
            Some(_) => {}
            None => {
                let child = self.level(index + 1).and_then(|c| c.child_of);
                if child.is_some() && list.hot.get() != child {
                    self.close_from(index + 1);
                }
            }
        }
    }

    fn hit(&self, point: POINT) -> Hit {
        let levels = self.levels.borrow().clone();
        for (index, level) in levels.iter().enumerate().rev() {
            if level.contains(point) {
                return Hit::List(index, level.item_at(point));
            }
        }
        if let Some(strip) = &self.strip {
            if strip.band_contains(point) {
                return Hit::Bar(strip.slot_at_screen(point));
            }
        }
        Hit::Outside
    }

    fn on_mouse_move(&self, point: POINT, cursor: POINT) {
        // Movimentos sem deslocamento real do cursor (gerados pelo sistema quando uma
        // lista aparece sob o cursor parado) não mudam a seleção feita pelo teclado.
        if self.last_mouse.replace(Some((cursor.x, cursor.y))) == Some((cursor.x, cursor.y)) {
            return;
        }
        match self.hit(point) {
            Hit::List(index, item) => {
                let Some(level) = self.level(index) else { return };
                if !level.tracking_leave.get() {
                    let mut track = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: level.hwnd.get(),
                        dwHoverTime: 0,
                    };
                    // SAFETY: estrutura local corretamente dimensionada.
                    unsafe { TrackMouseEvent(&mut track) };
                    level.tracking_leave.set(true);
                }
                // Ao entrar numa lista, as anteriores voltam a destacar o item que a abriu.
                if self.pending.get().is_some_and(|(p, _)| p < index) {
                    self.pending.set(None);
                }
                for ancestor in 0..index {
                    if let (Some(a), Some(child)) = (self.level(ancestor), self.level(ancestor + 1)) {
                        a.set_hot(child.child_of);
                    }
                }
                let has_child = self.depth() > index + 1;
                match item {
                    None if !has_child => level.set_hot(None),
                    None => {}
                    Some(item) if level.hot.get() != Some(item) => {
                        level.set_hot(Some(item));
                        let entry = &level.entries[item];
                        let child_item = self.level(index + 1).and_then(|c| c.child_of);
                        if matches!(entry.kind, EntryKind::Submenu(_)) && entry.enabled {
                            if child_item != Some(item) {
                                self.schedule(&level, index, Some(item));
                            }
                        } else if has_child {
                            self.schedule(&level, index, None);
                        }
                    }
                    Some(_) => {}
                }
            }
            Hit::Bar(Some(slot)) => {
                let Some(strip) = &self.strip else { return };
                if self.dropped.get() {
                    if strip.open_slot() != Some(slot) {
                        self.open_slot(slot, false);
                    }
                } else {
                    strip.set_state(Some(slot), None);
                }
            }
            Hit::Bar(None) | Hit::Outside => {
                let switch = self.switch.borrow().clone();
                if let Some(switch) = switch {
                    switch(point);
                }
            }
        }
    }

    fn on_mouse_leave(&self, hwnd: HWND) {
        let index = self.levels.borrow().iter().position(|l| l.hwnd.get() == hwnd);
        let Some(index) = index else { return };
        let Some(level) = self.level(index) else { return };
        level.tracking_leave.set(false);
        if self.depth() == index + 1 {
            level.set_hot(None);
            if self.pending.get().is_some_and(|(p, _)| p == index) {
                self.pending.set(None);
            }
        }
    }

    /// Whether click should be discarded.
    fn on_button_down(&self, point: POINT) -> bool {
        match self.hit(point) {
            Hit::List(index, Some(item)) => {
                if let Some(level) = self.level(index) {
                    level.set_hot(Some(item));
                }
                if self.level(index).is_some_and(|l| matches!(l.entries[item].kind, EntryKind::Submenu(_))) {
                    self.open_child(index, item, false);
                }
            }
            Hit::List(_, None) => {}
            Hit::Bar(Some(slot)) => {
                let open = self.strip.as_ref().and_then(|s| s.open_slot());
                if self.dropped.get() && open == Some(slot) {
                    end();
                } else {
                    self.dropped.set(true);
                    self.open_slot(slot, false);
                }
            }
            Hit::Bar(None) | Hit::Outside => end(),
        }
        true
    }

    fn on_button_up(&self, point: POINT) {
        if let Hit::List(index, Some(item)) = self.hit(point) {
            let command = self.level(index).is_some_and(|l| matches!(l.entries[item].kind, EntryKind::Command(_)));
            if command {
                self.activate(index, item);
            }
        }
    }

    fn on_wheel(&self, point: POINT, delta: i32) {
        if let Hit::List(index, _) = self.hit(point) {
            if let Some(level) = self.level(index) {
                level.scroll_by(-delta * 3 * SCROLL_BAND);
            }
        }
    }

    /// Navigation keys; `false` for unhandled keys.
    fn on_key(&self, vk: i32) -> bool {
        self.cues.set(true);
        if let Some(strip) = &self.strip {
            strip.repaint();
        }
        let Some((index, level)) = self.deepest() else { return self.on_bar_key(vk) };
        match vk {
            VK_DOWN | VK_UP => level.set_hot(level.step(vk == VK_DOWN)),
            VK_HOME => level.set_hot(level.selectable().first().copied()),
            VK_END => level.set_hot(level.selectable().last().copied()),
            VK_RIGHT => {
                let hot = level.hot.get();
                let submenu =
                    hot.filter(|&h| matches!(level.entries[h].kind, EntryKind::Submenu(_)) && level.entries[h].enabled);
                match submenu {
                    Some(item) => self.open_child(index, item, true),
                    None => self.step_bar(true),
                }
            }
            VK_LEFT => {
                if index > 0 {
                    self.close_from(index);
                } else {
                    self.step_bar(false);
                }
            }
            VK_RETURN => {
                if let Some(item) = level.hot.get() {
                    self.activate(index, item);
                }
            }
            VK_ESCAPE => {
                if index > 0 {
                    self.close_from(index);
                } else if let Some(strip) = &self.strip {
                    // Fecha a lista mas continua no modo de menu, com o menu selecionado.
                    let slot = strip.open_slot();
                    self.close_from(0);
                    self.dropped.set(false);
                    strip.set_state(slot, None);
                } else {
                    end();
                }
            }
            _ => return false,
        }
        true
    }

    /// Keys with no open lists, only selected bar.
    fn on_bar_key(&self, vk: i32) -> bool {
        let Some(strip) = self.strip.clone() else {
            end();
            return true;
        };
        let hot = strip.hot();
        match vk {
            VK_LEFT | VK_RIGHT => strip.set_state(strip.next_slot(hot, vk == VK_RIGHT), None),
            VK_DOWN | VK_UP | VK_RETURN => {
                if let Some(slot) = hot.or_else(|| strip.next_slot(None, true)) {
                    self.dropped.set(true);
                    self.open_slot(slot, true);
                    if vk == VK_UP {
                        if let Some((_, level)) = self.deepest() {
                            level.set_hot(level.step(false));
                        }
                    }
                }
            }
            VK_ESCAPE => end(),
            _ => return false,
        }
        true
    }

    /// Left/Right at first level selects neighboring bar menu.
    fn step_bar(&self, forward: bool) {
        let Some(strip) = self.strip.clone() else { return };
        if let Some(next) = strip.next_slot(strip.open_slot().or(strip.hot()), forward) {
            self.open_slot(next, true);
        }
    }

    /// Typed letter activates matching item/bar mnemonic.
    fn on_char(&self, c: char) {
        if c.is_control() {
            return;
        }
        let Some(lower) = c.to_lowercase().next() else { return };
        let Some((index, level)) = self.deepest() else {
            if let Some(strip) = self.strip.clone() {
                if let Some(slot) = strip.slot_by_mnemonic(lower) {
                    self.dropped.set(true);
                    self.open_slot(slot, true);
                }
            }
            return;
        };
        let matches: Vec<usize> =
            (0..level.entries.len()).filter(|&i| level.entries[i].mnemonic() == Some(lower)).collect();
        match matches.as_slice() {
            [] => {}
            [only] => {
                level.set_hot(Some(*only));
                self.activate(index, *only);
            }
            several => {
                let next = level
                    .hot
                    .get()
                    .and_then(|h| several.iter().position(|&i| i == h))
                    .map(|p| several[(p + 1) % several.len()])
                    .unwrap_or(several[0]);
                level.set_hot(Some(next));
            }
        }
    }

    /// Handles dequeued message; `true` means consumed.
    pub(super) fn translate(&self, msg: &MSG) -> bool {
        // SAFETY: consulta simples.
        if unsafe { IsWindow(self.owner) } == 0 {
            end();
            return false;
        }
        let ours = native::root_of(msg.hwnd) == self.owner || is_popup(msg.hwnd);
        match msg.message {
            WM_MOUSEMOVE | WM_NCMOUSEMOVE => {
                self.on_mouse_move(screen_point(msg), msg.pt);
                ours
            }
            WM_LBUTTONDOWN | WM_LBUTTONDBLCLK | WM_RBUTTONDOWN | WM_RBUTTONDBLCLK | WM_MBUTTONDOWN
            | WM_MBUTTONDBLCLK | WM_NCLBUTTONDOWN | WM_NCLBUTTONDBLCLK | WM_NCRBUTTONDOWN | WM_NCRBUTTONDBLCLK
            | WM_NCMBUTTONDOWN => {
                let swallow = self.on_button_down(screen_point(msg));
                swallow && ours
            }
            WM_LBUTTONUP | WM_RBUTTONUP | WM_NCLBUTTONUP | WM_NCRBUTTONUP => {
                self.on_button_up(screen_point(msg));
                ours
            }
            WM_MBUTTONUP | WM_NCMBUTTONUP => ours,
            WM_MOUSEWHEEL => {
                let delta = (hiword(msg.wParam) as i16 as i32) / WHEEL_DELTA;
                self.on_wheel(screen_point(msg), delta);
                true
            }
            WM_MOUSELEAVE if is_popup(msg.hwnd) => {
                self.on_mouse_leave(msg.hwnd);
                true
            }
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                let vk = msg.wParam as i32;
                if vk == VK_MENU || vk == VK_F10 {
                    suppress_key_up(vk);
                    end();
                } else if !self.on_key(vk) {
                    // Gera o WM_CHAR para os mnemônicos digitados sem Alt.
                    // SAFETY: tradução de uma mensagem válida recém-retirada da fila.
                    unsafe { TranslateMessage(msg) };
                }
                true
            }
            WM_CHAR | WM_SYSCHAR => {
                if let Some(c) = char::from_u32(msg.wParam as u32) {
                    self.on_char(c);
                }
                true
            }
            WM_KEYUP | WM_SYSKEYUP => {
                take_suppressed_key_up(msg.wParam as i32);
                true
            }
            _ => false,
        }
    }
}
