//! [`DataGrid`](super::DataGrid) export to CSV, TSV, HTML and clipboard.

use super::*;

impl DataGrid {
    // ------------------------------------------------------------------
    // Exportação
    // ------------------------------------------------------------------

    fn export_rows(&self) -> Vec<Vec<String>> {
        let columns = self.column_count();
        let count = self.row_count();
        (0..count).map(|row| (0..columns).map(|column| self.cell_value(row, column)).collect()).collect()
    }

    /// Displayed rows in CSV with headers and `separator` (`,` or `;`); fields containing separators, quotes or
    /// newlines are quoted. Virtual mode traverses the entire source.
    pub fn to_csv(&self, separator: char) -> String {
        let titles: Vec<String> = (0..self.column_count()).map(|c| self.column_title(c).unwrap_or_default()).collect();
        let mut out = String::new();
        for line in std::iter::once(titles).chain(self.export_rows()) {
            let fields: Vec<String> = line.iter().map(|f| csv_field(f, separator)).collect();
            out.push_str(&fields.join(&separator.to_string()));
            out.push_str("\r\n");
        }
        out
    }

    /// Displayed rows separated by tabs (spreadsheet paste format).
    pub fn to_tsv(&self) -> String {
        self.to_csv('\t')
    }

    /// HTML table of displayed rows, with programmatically defined row/cell styles.
    pub fn to_html(&self) -> String {
        let columns = self.column_count();
        let mut out = String::from("<table>\n<thead><tr>");
        for column in 0..columns {
            out.push_str(&format!("<th>{}</th>", html_escape(&self.column_title(column).unwrap_or_default())));
        }
        out.push_str("</tr></thead>\n<tbody>\n");
        for (row, cells) in self.export_rows().into_iter().enumerate() {
            out.push_str("<tr>");
            for (column, cell) in cells.iter().enumerate() {
                let style = self.cell_style(row, column);
                let mut css = String::new();
                if let Some(color) = style.foreground {
                    css.push_str(&format!("color:{};", color_css(color)));
                }
                if let Some(color) = style.background {
                    css.push_str(&format!("background:{};", color_css(color)));
                }
                if style.bold {
                    css.push_str("font-weight:bold;");
                }
                if style.italic {
                    css.push_str("font-style:italic;");
                }
                if css.is_empty() {
                    out.push_str(&format!("<td>{}</td>", html_escape(cell)));
                } else {
                    out.push_str(&format!("<td style=\"{css}\">{}</td>", html_escape(cell)));
                }
            }
            out.push_str("</tr>\n");
        }
        out.push_str("</tbody>\n</table>\n");
        out
    }

    /// Writes displayed rows to UTF-8 CSV with BOM (Excel recognizes accented characters).
    pub fn export_csv(&self, path: impl AsRef<std::path::Path>, separator: char) -> GuiResult<()> {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(self.to_csv(separator).as_bytes());
        std::fs::write(path.as_ref(), bytes)
            .map_err(|e| GuiError::InvalidArgument(format!("could not write {}: {e}", path.as_ref().display())))
    }

    /// Copies selected rows (all if none selected) to the clipboard as tab-separated text and HTML.
    pub fn copy_to_clipboard(&self) -> GuiResult<()> {
        let selected = self.selected_rows();
        let rows: Vec<usize> = if selected.is_empty() { (0..self.row_count()).collect() } else { selected };
        let columns = self.column_count();
        let mut text = String::new();
        let mut html = String::from("<table>");
        for row in rows {
            let cells: Vec<String> = (0..columns).map(|c| self.cell_value(row, c)).collect();
            text.push_str(&cells.iter().map(|c| csv_field(c, '\t')).collect::<Vec<_>>().join("\t"));
            text.push_str("\r\n");
            html.push_str("<tr>");
            for cell in &cells {
                html.push_str(&format!("<td>{}</td>", html_escape(cell)));
            }
            html.push_str("</tr>");
        }
        html.push_str("</table>");
        crate::system::Clipboard::set(&ClipboardContent::new().text(&text).html(&html))
    }
}
