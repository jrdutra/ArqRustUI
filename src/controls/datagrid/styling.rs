//! [`DataGrid`](super::DataGrid) row/cell styles and per-row icons.

use super::*;

impl DataGrid {
    // ------------------------------------------------------------------
    // Estilos
    // ------------------------------------------------------------------

    /// Formats a cell (in-memory mode; follows the row during sorting/reordering).
    pub fn set_cell_style(&self, row: usize, column: usize, style: CellStyle) {
        if let Some(r) = self.model.borrow_mut().row_mut(row) {
            r.cell_styles.insert(column, style);
        }
        self.redraw_row(row);
    }

    /// Formats a whole in-memory row; cell styles take precedence.
    pub fn set_row_style(&self, row: usize, style: Option<CellStyle>) {
        if let Some(r) = self.model.borrow_mut().row_mut(row) {
            r.style = style;
        }
        self.redraw_row(row);
    }

    /// Removes all programmatically defined row/cell styles.
    pub fn clear_styles(&self) {
        for row in self.model.borrow_mut().data.iter_mut() {
            row.style = None;
            row.cell_styles.clear();
        }
        self.refresh();
    }

    /// Effective cell style from [`set_row_style`](Self::set_row_style) and [`set_cell_style`](Self::set_cell_style),
    /// excluding the style function.
    pub fn cell_style(&self, row: usize, column: usize) -> CellStyle {
        self.model.borrow().static_style(row, column)
    }

    /// Formats visible cells via `(row, column, text) -> style`, useful for rules ("negative values in red") and
    /// virtual sources. Combines with programmatic styles; `None` disables.
    pub fn set_cell_style_provider(
        &self,
        provider: Option<impl Fn(usize, usize, &str) -> Option<CellStyle> + 'static>,
    ) {
        let previous = std::mem::replace(
            &mut self.model.borrow_mut().style_provider,
            provider.map(|p| Rc::new(p) as StyleProvider),
        );
        drop(previous);
        self.refresh();
    }

    // ------------------------------------------------------------------
    // Ícones
    // ------------------------------------------------------------------

    pub(super) fn row_image_index(&self, core: &Rc<WidgetCore>, row: usize) -> i32 {
        let provider = self.model.borrow().icon_provider.clone();
        if let Some(provider) = provider {
            let size = ManagedImages::logical_size(core).unwrap_or(DEFAULT_ICON_SIZE);
            return provider(row)
                .and_then(|icon| ManagedImages::index_of(core, icon, size, listview::attach_images).ok())
                .unwrap_or(I_IMAGENONE);
        }
        self.model.borrow().row(row).map_or(I_IMAGENONE, |r| r.icon)
    }

    /// Shows an icon in the row's first column (in-memory mode).
    ///
    /// The icon follows its row when reordered.
    pub fn set_row_icon(&self, row: usize, icon: &impl IconSource) -> GuiResult<()> {
        if self.model.borrow().row(row).is_none() {
            return Err(GuiError::InvalidArgument(format!("row {row} does not exist in in-memory mode")));
        }
        let size = ManagedImages::logical_size(&self.core).unwrap_or(DEFAULT_ICON_SIZE);
        let index = ManagedImages::index_of(&self.core, icon, size, listview::attach_images)?;
        if let Some(r) = self.model.borrow_mut().row_mut(row) {
            r.icon = index;
        }
        self.redraw_row(row);
        Ok(())
    }

    /// Removes the row icon (in-memory mode).
    pub fn clear_row_icon(&self, row: usize) {
        if let Some(r) = self.model.borrow_mut().row_mut(row) {
            r.icon = I_IMAGENONE;
        }
        self.redraw_row(row);
    }

    /// Sets icons via an on-demand `row -> icon` function for visible rows, useful for virtual sources. Overrides
    /// [`set_row_icon`](Self::set_row_icon); `None` disables.
    ///
    /// ```no_run
    /// # use arqrustui::prelude::*;
    /// # fn demo(grid: &DataGrid) -> GuiResult<()> {
    /// grid.set_virtual_source(1_000_000, |row, column| format!("{row}:{column}"));
    /// grid.set_icon_provider(Some(|row: usize| Some(if row % 2 == 0 { &icons::FILE } else { &icons::FOLDER })))?;
    /// # Ok(()) }
    /// ```
    pub fn set_icon_provider(
        &self,
        provider: Option<impl Fn(usize) -> Option<&'static IconAsset> + 'static>,
    ) -> GuiResult<()> {
        if provider.is_some() {
            let size = ManagedImages::logical_size(&self.core).unwrap_or(DEFAULT_ICON_SIZE);
            ManagedImages::ensure(&self.core, size, listview::attach_images)?;
        }
        let previous =
            std::mem::replace(&mut self.model.borrow_mut().icon_provider, provider.map(|p| Rc::new(p) as IconProvider));
        drop(previous);
        self.refresh();
        Ok(())
    }

    /// Logical row icon size (default: 16). Reloads icons already used.
    pub fn set_icon_size(&self, size: i32) -> GuiResult<()> {
        ManagedImages::resize(&self.core, size.max(1), listview::attach_images)?;
        self.refresh();
        Ok(())
    }

    /// Row image index (`None` without an icon).
    pub fn row_image(&self, row: usize) -> Option<i32> {
        let index = self.row_image_index(&self.core, row);
        (index >= 0).then_some(index)
    }
}
