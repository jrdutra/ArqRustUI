# Component guide

Rustdoc is in English, including examples and macro-generated documentation.
`Canvas::with_gdiplus` applies DPI scaling once and preserves canvas clipping and
transforms. The `modern_controls` example uses responsive layouts; New in the
`demo` and `menus` examples uses `icons::FILE`.

*Português: [COMPONENTS.md](COMPONENTS.md).*

Summary reference. The full reference, with every method, is in rustdoc (`cargo doc --open`).

## Conventions for every control

```rust
let control = Type::builder()       // every control with options has a builder
    .text("…")                      // initial text
    .bounds(x, y, width, height)    // or .position(x, y) + .size(w, h) — logical units
    .parent(&window_or_container)   // REQUIRED
    .enabled(true).visible(true).tab_stop(true)
    .font(Font::new("Segoe UI", 10.0)).tooltip("hint")
    .build()?;                      // GuiResult<Type>
```

`Widget` methods (all controls and `Window`): `text`/`set_text`, `bounds`/`set_bounds`, `position`/`set_position`, `size`/`set_size`, `client_size`, `show`/`hide`/`set_visible`/`is_visible`, `set_enabled`/`is_enabled`, `focus`/`has_focus`, `set_tab_stop`, `set_font`/`font`, `set_foreground`/`set_background`, `set_cursor`, `set_tooltip`, `set_context_menu`, `invalidate`, `dpi`/`scale_factor`, `set_accessible_name`/`set_accessible_description`, `destroy`/`is_alive`, `handle`, and the `on_*` events described in [EVENTS.en.md](EVENTS.en.md).

## Window

Without `.icon(...)`, every `Window` uses `icons::DEFAULT_WINDOW_ICON`, including
secondary windows and the library's own dialogs. The logo stays available without the
`icons` feature; `.icon(&other)` or `set_icon(&other)` take precedence over this default
and keep being reapplied after DPI changes. Icon resources of the executable file
belong to each application's build.

| Type | Construction | Main methods |
|---|---|---|
| `Window` | `Window::builder().title().size().position().centered().resizable().minimizable().maximizable().borderless().topmost().tool_window().dialog_frame().opacity().owner().icon().min_size().max_size().accept_files().close_on_escape().background().right_to_left().build()?` | `close`, `maximize`/`minimize`/`restore`, `set_fullscreen`, `center`, `move_to_monitor`, `monitor`, `set_opacity`, `set_topmost`, `set_icon`, `set_menu`, `set_menu_strip`/`remove_menu_strip`/`menu_strip`, `system_menu`, `set_layout`/`relayout`, `set_min_size`/`set_max_size`, `add_shortcut`, `show_modal`, `on_close_request` (`false` cancels), `on_paint`, DWM effects (`set_dark_title_bar`, `set_corner_style`, `set_border_color`, `set_caption_color`, `set_backdrop`, `extend_frame_into_client`) |

## Controls provided by Windows

| Type | Native class | Builder options | Own methods |
|---|---|---|---|
| `Button` | `BUTTON` | `default`, `cancel`, `style`; default height `DEFAULT_CONTROL_HEIGHT` | `click`, `set_style`; icon: builder `icon`, `icon_size` (default: automatic, 60% of the height), `icon_position`; `set_icon`, `clear_icon`, `set_icon_position`, `set_icon_size(Some(n) \| None)`, `icon_size()` (effective side), `is_icon_native`. Icon and text form a centered group, with a gap of ~1/3 of the icon between them and an 8-unit margin from the edge; text that does not fit becomes an ellipsis |
| `Label` | `STATIC` | `align`, `no_prefix`, `single_line` (one line vertically centered, to line up with fields) | —; icon: builder `icon`, `icon_size`, `icon_position`; `set_icon`, `clear_icon` |
| `TextBox` | `EDIT` | default height `DEFAULT_CONTROL_HEIGHT` with vertically centered text (single line); `multiline`, `word_wrap`, `read_only`, `numeric`, `align`, `placeholder`, `max_length` | `set_placeholder`, `select_all`, `selection`/`set_selection`, `replace_selection`, `append`, `set_read_only`, `undo`, `cut`/`copy`/`paste`, `is_modified`/`set_modified`, `line_count` |
| `PasswordBox` | `EDIT` + `ES_PASSWORD` | `placeholder`, `max_length` | `password`, `set_revealed` + edit methods |
| `RichTextBox` | `RICHEDIT50W` | `read_only`, `word_wrap` | `format_selection`, `format_all`, `selection_style`, `set_alignment`, `set_list_style(ListStyle)`/`list_style` (bullets, numbers, letters, Roman numerals), `insert_rtf`, `insert_table(rows, widths, header)`, `insert_image(&PlusImage, w, h)`, `rtf`/`set_rtf`, `find` + edit methods |
| `CheckBox` | `BUTTON` | `checked`, `three_state` | `is_checked`/`set_checked`, `state`/`set_state` |
| `RadioButton` | `BUTTON` | `checked`, `group_start` | `is_checked`/`set_checked` |
| `ToggleButton` | `BUTTON` + `BS_PUSHLIKE` | `checked` | `is_checked`/`set_checked` |
| `ComboBox` | `COMBOBOX` | `editable`, `sorted`, `items`, `selected`, `with_icons` (editable ones only); default height `DEFAULT_CONTROL_HEIGHT` (28, the same as `Button`), kept for any font and DPI, with centered text | `add_item`, `insert_item`, `remove_item`, `clear`, `count`, `item_text`, `selected_index`/`set_selected_index`, `selected_text`; icons (non-editable, or editable with `with_icons(true)`): `add_item_with_icon`, `set_item_icon`, `clear_item_icon`, `item_image` |
| `ListBox` | `LISTBOX` | `multi_select`, `sorted`, `items` | `add_item`, `insert_item`, `remove_item`, `clear`, `count`, `item_text`, `selected_index`, `selected_indices`, `set_selected_index`, `set_item_selected`; with `with_icons(true)`: `add_item_with_icon`, `set_item_icon`, `clear_item_icon`, `item_image` |
| `GroupBox` | `BUTTON` + `BS_GROUPBOX` | — | `set_layout` (accepts children) |
| `ProgressBar` | `msctls_progress32` | `range`, `value`, `vertical`, `marquee` | `set_range`, `value`/`set_value`, `advance`, `set_marquee`, `set_state` |
| `Slider` | `msctls_trackbar32` | `range`, `value`, `vertical`, `ticks` | `set_range`, `value`/`set_value`, `set_page_size` |
| `ScrollBar` | `SCROLLBAR` | `vertical`, `range`, `page`, `value` | `configure`, `value`/`set_value` |
| `UpDown` | `msctls_updown32` | — | `set_range`, `value`/`set_value` |
| `DatePicker` / `TimePicker` | `SysDateTimePick32` | `long_format` (date); default height `DEFAULT_CONTROL_HEIGHT`, centered text | `value`/`set_value`, `set_format` |
| `MonthCalendar` | `SysMonthCal32` | — | `selected_date`/`set_selected_date`, `minimum_size` |
| `Hyperlink` | `SysLink` | text with `<a href="…">` | `on_link_click`, `open_links_in_browser` |
| `Tooltip` | `tooltips_class32` | — | `Tooltip::set`, `remove`, `set_enabled` (or `widget.set_tooltip`) |
| `StatusBar` | `msctls_statusbar32` | `StatusBar::new(&window)?` | `set_parts`, `set_part_text`, `part_text`, `height`, `set_part_icon`, `clear_part_icon`, `has_part_icon` |
| `ToolBar` | `ToolbarWindow32` | `ToolBar::new(&window)?` | `add_button`, `add_toggle_button`, `add_separator`, `set_button_enabled`, `set_button_checked`, `trigger`, `height` (icon: `Some(&icons::X)` or `Some(&Icon)`) |
| `ListView` | `SysListView32` | `single_selection`, `grid_lines`, `checkboxes` | `add_column`, `column_width`, `add_row`/`insert_row`, `cell`/`set_cell`, `remove_row`, `clear`, `row_count`, `selected_rows`, `set_row_selected`, `is_row_checked`, `set_image_list`, `set_row_image`, `sort_by_column`, `set_row_icon`, `clear_row_icon`, `row_image`, `set_icon_size` |
| `TreeView` | `SysTreeView32` | `checkboxes` | `add_root`, `add_child`, `remove`, `clear`, `roots`, `children`, `parent_of`, `item_text`, `item_path`, `expand`/`collapse`, `selected`/`select`, `is_checked`/`set_checked`, `set_image_list`, `set_item_image`, `set_item_icon`, `set_item_icons`, `clear_item_icon`, `item_image`, `set_icon_size` |
| `TabControl` | `SysTabControl32` | — | `add_tab` (returns the tab's `Panel`), `remove_tab`, `tab_count`, `page`, `tab_title`/`set_tab_title`, `selected_index`/`set_selected_index`, `set_tab_icon`, `clear_tab_icon`, `tab_image`, `tab_at`, `tab_bounds`; detachable tabs: `set_detachable(true)` (drag out), `detach_tab(i)` → `DetachedTab { title, page, window }`, `attach_tab(&window)`, `detached_tabs`, `on_tab_detached`, `on_tab_attached` (closing the window gives the tab back) |
| `HeaderControl` | `SysHeader32` | — | `add_item`, `remove_item`, `item_count`, `item_width`, `set_sort_indicator` |
| `Rebar` | `ReBarWindow32` | `Rebar::new(&window)?` | `add_band`, `remove_band`, `band_count`, `height` |
| `Pager` | `SysPager` | `horizontal` | `set_child`, `scroll_position`/`set_scroll_position` |
| `HotKeyBox` | `msctls_hotkey32` | — | `shortcut`/`set_shortcut` |
| `IpAddressBox` | `SysIPAddress32` | — | `address`/`set_address`, `clear` |
| `Animation` | `SysAnimate32` | — | `open`, `play`, `stop` |

## Controls implemented by the library

| Type | How it is built | Main methods |
|---|---|---|
| `Panel` | the library's window class | `set_layout`, `on_paint`, `set_double_buffered`; builder: `border`, `background`, `focusable` |
| `SpinButton` | `EDIT` + attached `UpDown` | `set_range`, `value`/`set_value`, `on_value_changed` |
| `PictureBox` | panel with its own drawing | builder `icon`, `icon_size`, `background`; `set_icon`, `set_image(PlusImage)`, `set_svg(SvgImage)` (sharp at any size), `set_icon_size`, `clear`, `has_content` (above 256 px it draws the icon's `.png`) |
| `SplitContainer` | two `Panel`s + draggable splitter | `first()`, `second()`, `split_position`/`set_split_position`, `set_min_sizes`, `on_split_moved`; builder: `stacked`, `split_position`, `min_sizes` |
| `DataGrid` | virtual ListView + Rust model + overlaid editor | `add_column`, `set_rows`, `add_row`, `remove_row`, `clear`, `set_virtual_source`, `cell`/`set_cell`, `rows`, `sort_by`, `selected_rows`, `set_column_read_only`, `set_validator`, `begin_edit`/`commit_edit`/`cancel_edit`, `on_cell_edited`; builder: `editable`, `sortable`, `single_selection`; icons: `set_row_icon` (follows sorting), `set_icon_provider`, `set_icon_size`, `row_image`; styles: `set_cell_style(row, column, CellStyle::new().foreground().background().bold().italic())`, `set_row_style`, `set_cell_style_provider`, `clear_styles`; `set_column_kind(c, ColumnKind::Check)` + `toggle_check`, `parse_check`; editable virtual source: `set_virtual_editor`; `set_filter`/`set_text_filter`/`has_filter`; `set_group_by(Some(column))`, `groups`, `set_group_collapsed`, `on_group_toggled`; `to_csv`/`to_tsv`/`to_html`, `export_csv(path)`, `copy_to_clipboard` |
| `CommandBar` | `Panel` drawn by the library + hosted `EDIT` for search; menus drawn like `MenuStrip` | `CommandBar::builder().parent(&w).build()?` (default height `COMMAND_BAR_HEIGHT` = 36; builder `style(MenuStyle)`, `bottom_border`); `add_button(CommandButton)`, `add_separator`, `add_spacer` (pushes the rest to the right), `add_label`, `add_search(placeholder, width, on_search)` → `TextBox` (Enter searches, Esc clears, `×` clears), `add_widget(&child_control, width)`, `clear`, `len`, `height`, `set_style`/`style`, `overflow_count`, `open_overflow`. `CommandButton::new(text)` → `icon`, `icon_only`, `tooltip`, `on_click`, `toggle(checked)` + `on_toggled`, `menu(&Menu)` (`▾` button), `split(&Menu)` (action + arrow), `enabled`. `CommandItem` → `set_enabled`, `set_checked`/`is_checked`, `set_text`/`text`, `set_icon`/`clear_icon`, `set_tooltip`, `set_visible`, `is_in_overflow`, `trigger`, `open_menu`. What does not fit goes to the `»` button; with a button's menu open, hovering another menu button switches the list; Tab into the bar, arrows, Enter/Space and Down/F4 from the keyboard. Customization: `customize()` (box with Add/Remove/Move up/Move down/Reset), `layout()`/`set_layout(&CommandBarLayout)` (savable text), `set_default_layout`, `reset_layout`; `set_reorderable(true)` lets buttons be dragged. Hosted controls that do not fit go to `»`. |
| `InfoBar` | `Panel` drawn by the library + action `Button`s | builder `severity(Severity)`, `title`, `message`, `closable` (default height `INFO_BAR_HEIGHT` = 44); `add_action(text, on_click)` → `Button`; `open`, `open_with(severity, title, message)`, `close`, `is_open`, `on_closed`; `set_severity`, `set_title`, `set_message`, `set_closable`. Opening/closing shows/hides it and lays out the container again. The accessible name announces severity, title and message. |
| `Toast` | its own `WS_POPUP` window, never activated, translucent only while fading | `Toast::new(title)` → `message`, `severity`, `duration`, `persistent`, `action(text, f)`, `on_dismissed`, `show(&window)` → `ToastHandle` (`close`, `is_open`); `Toast::dismiss_all(&window)`, `Toast::count(&window)`. Bottom-right corner, stacked, they follow the window; the timer pauses while the cursor is over them. |
| `PropertyGrid` | two-column `DataGrid` with typed values | `PropertyGrid::new(&parent, Rect)?`, `add_category`, `add_property`, `value`/`set_value`, `property_names`, `activate`, `on_property_changed`, `set_choice_mode(ChoiceMode::DropDown \| Cycle)`, `set_bool_labels(Some(("On", "Off")))` |
| `ToggleSwitch` | drawn panel (animated sliding knob) | builder `text` (header), `on`, `state_texts(on, off)`; `is_on`/`set_on`/`toggle`, `set_header`, `state_text`, `on_toggled(\|on\| ..)`; Space/Enter; UIA Toggle |
| `NumberBox` | hosted `EDIT` + drawn ▲/▼ buttons | builder `value`, `range`, `step`, `large_step`, `decimals`, `decimal_comma`; `value`/`set_value`, `set_range`/`range`, `commit`, `text_box`, `on_value_changed`; ↑/↓, PgUp/PgDn, Enter evaluates expressions (`numberbox::evaluate`); UIA RangeValue + Value |
| `RatingControl` | drawn panel (stars) | builder `maximum`, `value`, `allow_half`, `read_only`; `value`/`set_value`, `maximum`, `set_read_only`, `star_bounds`, `on_value_changed`; arrows, Home/End, 0–9; UIA RangeValue |
| `Card` | drawn panel (rounded corners) | builder `title`, `subtitle`, `clickable`, `padding`; `content_bounds` (where to put children), `set_title`/`set_subtitle`, `on_click` |
| `Expander` | drawn header + content `Panel` | builder `header`, `expanded`, `content_height`; `content()`, `is_expanded`/`set_expanded`/`toggle` (animated height, lays out the parent again), `preferred_height`, `set_content_height`, `on_expanded_changed`; UIA ExpandCollapse |
| `BreadcrumbBar` | drawn panel | builder `items`; `items`, `set_items`, `push`, `truncate`, `len`, `item_bounds`, `hidden_items`, `on_item_click(\|index\| ..)`; when space runs out, the first items go into "…" (menu) |
| `Chart` | drawn panel (GDI+) | builder `kind(ChartKind::{Line, Area, Bar, StackedBar, Pie})`, `title`, `categories`, `series(ChartSeries::new(name, &values).with_color(..))`, `legend`, `animated`; `set_kind`, `set_series`, `add_series`, `set_values`, `set_categories`, `set_value_format`, `hit_test`, `item_center`, `summary` (accessible description), `on_point_click(\|series, index\| ..)` |
| `TitleBar` | drawn panel + the window's non-client area | `TitleBar::attach(&window)?` (removes the system frame); `height`/`set_height`, `add_control(&child, width)`, `press(CaptionButton)`, `button_bounds`; put it at the top of the layout with `LayoutParams::new().height(bar.height())` |
| `DockHost` / `DockPane` | drawn panels + content `Panel`s | `DockHost::builder()...build()?`, `center()`, `add_pane(title, DockSide::{Left, Right, Bottom})` → `DockPane` (`content`, `float`, `dock`, `dock_to`, `toggle_floating`, `close`, `show`, `is_floating`/`is_docked`/`is_closed`, `floating_window`); `zone_size`/`set_zone_size`, `splitter_bounds`, `arrange` |
| `PrintPreview` / `PrintPreviewDialog` | drawn panel / `Dialog` | `PrintPreview::builder().document(doc).zoom(PreviewZoom::FitPage)`; `set_document`, `refresh`, `page`/`set_page`, `next_page`/`previous_page`, `zoom`/`set_zoom`, `zoom_in`/`zoom_out`, `page_bounds`, `on_page_changed`; `PrintPreviewDialog::new(doc).show(&window)?` → printed pages (`create` returns a `PrintPreviewWindow` to show it without blocking) |

## Icons (`arqrustui::icons`, `icons` feature)

392 embedded icons in 22 categories (from files, folders and 66 file types by extension to social networks, emoji and hand gestures), each one in `.ico` (16, 24, 32, 48, 64, 128 and 256 px) and `.png`; 79 also in `.svg`. Sheet with all of them: `assets/icons/preview.png`.

| Item | Use |
|---|---|
| Constants | `icons::FOLDER`, `icons::SAVE`, `icons::DATABASE`… (canonical name in upper case) |
| Social networks | `icons::YOUTUBE`, `INSTAGRAM`, `FACEBOOK`, `WHATSAPP`, `X_TWITTER`, `YOUTUBE_SHORTS`…; 58 designs in `IconCategory::Social`. `GITHUB_SOCIAL`, `GITLAB_SOCIAL` and `THREADS_ALTERNATE` keep separate variants of the earlier logos. Origin and visual limitations in `assets/icons/social/ORIGEM.md` (Portuguese). |
| Search | `icons::find("pasta-aberta")` accepts the name, an alias and the original file names (`"05_banco_de_dados"`); `icons::all()`, `icons::in_category(IconCategory::Files)` |
| File type | `icons::for_path("photos/beach.JPG")` → `JPG_FILE`, `icons::for_extension(".pdf")` → `PDF_FILE` (case-insensitive, with or without the dot; `htm`, `yml`, `tif`, `mjs`, `hpp`, `tgz`… use the icon of the common variant); `None` when there is no icon — `.unwrap_or(&icons::FILE)` |
| `IconAsset` | `name`, `category`, `aliases`, `ico_bytes`/`png_bytes`/`svg_bytes`, `sizes`, `png_size`, `icon(px)` (per-thread cache), `icon_for(&widget, logical)`, `image()` → `PlusImage`, `save(path)` (format from the extension), `IconAsset::new(..)` for custom icons |
| `IconSource` | accepted by every method that shows icons; implemented by `Icon` and `IconAsset` |
| `Icon` | `from_ico_bytes(bytes, px)`, `disabled()`, `size()` besides `system`/`from_file` |
| `ImageList` | `add(&impl IconSource)`, `icon_size`, `len` |
| `IconPosition` | `Left`, `Right`, `Top`, `Bottom`, `Center` (buttons and labels) |

## Layouts (`arqrustui::layout`)

| Type | Use |
|---|---|
| `StackLayout::vertical()` / `::horizontal()` | stack; `.spacing(n).padding(Thickness)`, `add`, `add_with(item, LayoutParams)`; recommended values: `layout::SPACING` (8) between controls and `layout::MARGIN` (12) at the edges |
| `GridLayout::new(&columns, &rows)` | grid with `Track::Fixed(n)`, `Track::Auto`, `Track::Star(weight)`; `.gaps(c, r)`, `add(item, row, column)`, `add_span`, `add_row` |
| `FlowLayout::new()` | flow with line wrapping; `.gaps(h, v)` |
| `DockLayout::new()` | docking: `add(item, Dock::Top/Bottom/Left/Right/Fill)` |
| `AnchorLayout::new(design_size)` | Windows Forms–style anchors: `add(&widget, Anchors::BOTTOM_RIGHT)` |
| `AbsoluteLayout::new()` | fixed rectangles (scaled by DPI) |
| `SplitLayout::horizontal(a, b)` / `::vertical(a, b)` | fixed proportional split: `.ratio(0.3).gap(6)` |
| `LayoutParams::new()` | `.width()`, `.height()`, `.size()`, `.min_size()`, `.max_size()`, `.margin()`, `.weight()`, `.align_h()`, `.align_v()` |
| `Spacer` | empty item (with `weight` it pushes its neighbors) |

Layouts nest: `outer.add_with(inner, LayoutParams::new().height(30))`. Hidden controls take no space. Attach with `window.set_layout(layout)`, `panel.set_layout(layout)` or `group.set_layout(layout)`.

## Menus (`arqrustui::menus`)

`Menu::new()?` → `on_highlight_changed(\|item\| ..)` (item highlighted by mouse/keyboard; `None` when leaving), `add_item`, `add_item_with_shortcut`, `add_check_item`, `add_radio_item` (consecutive radio items form a group), `add_separator`, `add_submenu`, `add_submenu_with_icon`, `remove`, `clear`, `on_open(|menu| ..)` (called before the menu is shown; also before a shortcut is checked in the `MenuStrip`). `MenuItem` → `set_enabled`, `set_checked`, `set_text`, `text`, `set_icon`, `trigger`. `window.system_menu()?` returns the system menu so items can be added to it.

The same `Menu`s serve two bars:

| Bar | Creation | Appearance |
|---|---|---|
| `MenuBar` | `MenuBar::new()?` → `add_menu`; `window.set_menu(&bar)?` | Windows bar (`HMENU`), system colors |
| `MenuStrip` | `MenuStrip::new()?` → `add_menu`, `add_menu_with_icon`; `window.set_menu_strip(&strip)?` | drawn by the library: light/dark/high-contrast theme, icons at any DPI |

`MenuStrip` → `set_menu_text`/`menu_text`, `set_menu_enabled`/`is_menu_enabled`, `set_menu_icon`/`clear_menu_icon`, `remove_menu`, `clear`, `len`, `set_style(Option<MenuStyle>)`/`style`, `height`, `open(index)`, `close`, `is_open`, `overflow_count`. `window.remove_menu_strip()`, `window.menu_strip()`. The strip lives in the non-client area (the client area and layouts do not change). Keyboard: Alt or F10 enter menu mode; Alt+letter opens a menu; arrows, Home/End, Enter, Esc and underlined letters navigate. Menus that do not fit go to the `»` button. `MenuStyle::{light, dark, high_contrast, system, from_theme(&theme), with_accent(color)}`; without an explicit style, the bar follows the window theme.

`ContextMenu::new()?` (same methods as `Menu`) → `widget.set_context_menu(&menu)` or `menu.show(&widget, point)`; `set_drawn(true)` uses the `MenuStrip` drawing (does not block; `close()` closes it).

## Dialogs (`arqrustui::dialogs`)

| Type | Use | Returns |
|---|---|---|
| `MessageBox` | `info`, `warning`, `error`, `confirm`, `show(&owner, title, text, buttons, icon)` | `()`, `bool` or `MessageResult` |
| `OpenFileDialog` | `.title().filter(name, "*.ext").folder().file_name().show(&owner)?` / `.show_multiple` | `Option<PathBuf>` / `Vec<PathBuf>` |
| `SaveFileDialog` | same + `.default_extension()` | `Option<PathBuf>` |
| `FolderPicker` | `.title().folder().show(&owner)?` | `Option<PathBuf>` |
| `ColorDialog` | `.initial(color).show(&owner)?` | `Option<Color>` |
| `FontDialog` | `.initial(&font).color(color).show(&owner)?` | `Option<FontChoice>` |
| `PrintDialog` | `.show(&owner)?` → `printer.print("doc", \|page, canvas\| …)`; `.show_with(&owner, &PrintOptions::new().page_count(n).allow_selection(true))?` → `printer.range()`, `printer.print_document(&PrintDocument)` | `Option<PrinterContext>` |
| `PageSetupDialog` | `.margins().show(&owner)?` | `Option<PageSetup>` |
| `FindDialog` | `FindDialog::find(&owner, text, \|event\| …)?` / `::replace` — **modeless** | `FindDialog` |
| `TaskDialog` | `.title().heading().content().icon().button(id, text).command_links().verification().show(&owner)?` | `TaskDialogResult` |
| `Dialog` | `Dialog::new(&owner, title, w, h)?`; controls with `.parent(dialog.window())`; `close_with(result)`; `show_modal()?` or `show_modeless(\|r\| …)` | `DialogResult` |
| `Modal` (web style: dimmed backdrop + card) | `Modal::builder().title().message().content_height(n).width(w).button(text, DialogResult::Cancel).primary_button(text, DialogResult::Ok).close_on_escape().close_on_backdrop().close_button().dim(0.4).build(&owner)?`; controls in `modal.content()`; `show()?` (blocks) or `show_then(\|r\| ..)?`; `close_with`, `on_close_request(\|r\| bool)` (validation), `on_closed`, `buttons`, `close_button`, `card`/`backdrop`, `card_bounds`, `shake`; shortcuts `Modal::alert(&owner, title, text)?`, `Modal::confirm(..)? -> bool` | `DialogResult` |

`Ok(None)` (or an empty vector) means the user cancelled.

## Graphics (`arqrustui::graphics`)

- `Canvas` (GDI): `clear`, `line`, `rect`, `fill_rect`, `round_rect`, `ellipse`, `polygon`, `polyline`, `path`, `text`, `text_in_rect`, `text_centered`, `text_line` (one line with ellipsis), `measure_text`, `gradient_rect`, `draw_bitmap`, `draw_bitmap_alpha`, `draw_icon`, `focus_rect`, `clip_rect`, `clip_path`, `reset_clip`, `save`/`restore`, `translate`/`rotate`/`scale`/`reset_transform`, `pixel`, `with_gdiplus`.
- `PlusGraphics` (GDI+): `clear`, `line`, `draw_rect`, `fill_rect`, `draw_ellipse`, `fill_ellipse`, `draw_polygon`, `fill_polygon`, `draw_path`, `fill_path`, `gradient_rect`, `draw_image`, `text`, `translate`/`rotate`/`scale`, `clip_rect`.
- `Direct2DSurface::attach(&panel, |d2d| …)?`; `D2DContext`: `clear`, `line`, `fill_rect`, `draw_rect`, `rounded_rect`, `ellipse`, `path`, `gradient_rect`, `text`, `push_transform`/`pop_transform`, `translate`/`rotate`/`scale`, `push_clip`/`pop_clip`, `set_antialias`.
- Vectors (`graphics::vector`): `Figure::{new, rect, ellipse, pie}` + `Segment::{Line, Cubic}`, `Paint::{Solid, Linear, Radial}`, `Stroke { width, join, cap, miter_limit, dashes }`, `Matrix::{translate, scale, rotate, multiply, inverse}`; `PlusGraphics::{fill_figures, stroke_figures, with_matrix}`.
- SVG: `SvgImage::{parse, from_bytes, from_file}` → `size`, `view_box`, `with_current_color`, `render(w, h)` → `PlusImage`, `draw(&PlusGraphics, x, y, w, h)`; `canvas.draw_svg(&svg, Rect)`; `icons::X.svg_image()`.
- Animations (`motion`, in the prelude): `Easing::{Linear, EaseIn, EaseOut, EaseInOut, Overshoot, CubicBezier(..)}`, `motion::animate(duration, easing, \|t\| ..)` / `animate_value(from, to, ..)` → `Transition` (`on_finished`, `cancel`, `finish`, `is_running`), `animate_bounds(&w, Rect, ..)`, `fade_window`, `show_animated`/`hide_animated(&w, ShowEffect::{Fade, SlideDown, ..}, duration)`, `lerp`/`lerp_rect`/`lerp_color`, `animations_enabled`/`set_animations_enabled`.
- `Font::new(family, points).bold().italic()`, `Font::system()`, `Font::load_file(path)`; `Pen::new(color, width)`; `Path::new().move_to().line_to().bezier_to().close()`; `Bitmap::from_file`, `Bitmap::render`, `Bitmap::from_icon`; `PlusImage::from_file`, `PlusImage::from_bytes`, `PlusImage::to_icon`; `canvas.draw_icon(&impl IconSource, origin, size)`.

## Themes (`arqrustui::themes`)

`Theme::light()`, `Theme::dark()`, `Theme::system()`, `.with_font()`, `.with_accent()`, `theme.apply(&window)`; `SystemTheme::current()`; `Style::new().font().foreground().background().apply_to(&widget)`; `ButtonStyle::from_accent(color, text)`.

## System (`arqrustui::system`)

`Clipboard::{set, set_text, text, has_text, set_html, html, has_html, set_image, image, has_image, png, metafile, has_metafile, set_metafile, set_files, files, set_data, data, has_format, format_names, sequence_number, clear}` with `ClipboardContent::new().text().html().image()?.image_pixels()?.metafile().files().custom()` (several formats at once; images also as `CF_DIBV5` and `PNG`) · `Metafile::{record(w, h, \|canvas\| ..), from_bytes, bytes, size, render}` · `ClipboardWatcher::new(\|\| ..)?` · `SystemNotification::new(title).body().attribution().image().hero_image().button(label, argument).text_input(id, hint).selection_input(id, options).button_for_input(..).progress(..).tag().group().header().scenario(NotificationScenario::..).suppress_popup().launch_argument().silent().long_duration().on_activated_with(\|NotificationActivation\| ..).on_dismissed().show()?` → `SystemNotificationHandle::{hide, update_progress}`; `.show_at(when)?` → `ScheduledNotification::cancel`; `SystemNotification::{set_app_id, app_id, is_supported, remove, remove_group, clear_all, launched_by_notification}`; `NotificationActivator::register(clsid, \|activation\| ..)` (click with the program closed) · `TrayIcon::new(&icon, tip)?` (`on_click`, `set_menu`, `show_notification`) · `shell::{open, open_with_verb, show_in_explorer, known_folder, associated_program}` · `Monitor::{all, primary, from_point}` · `Cursor::{system, from_file, position}` · `WindowsVersion::current()` · `accessibility::{high_contrast, screen_reader_active, text_scale_factor, animations_enabled}` · `version::{touch_available, max_touch_points}`.

## Accessibility, input and i18n

- Drawn controls that are accessible without configuration: `MenuStrip` (menu bar → items with ExpandCollapse; lists → items with Invoke/Toggle/ExpandCollapse), `CommandBar` (buttons with Invoke, Toggle or ExpandCollapse; split button as SplitButton), `PropertyGrid` (categories as groups; properties with Value, booleans with Toggle). `set_accessible_name`/`set_accessible_description` change the root's name/description.

- `accessibility::{set_custom_role, announce, announce_with, set_tab_order, show_focus_indicators}`; `widget.set_accessible_name(..)`.
- Accessible model for controls drawn by the application: `set_accessible_model(&w, impl AccessibleModel)` with `root()`, `children()` (`AccessibleItem::new(id, role, name).parent().bounds().invokable().toggle().expanded().value().range(AccessibleRange).selection().selected().grid().cell().scroll(AccessibleScroll).text().live()`) and `perform(id, AccessibleAction)`; `raise_accessible_event(&w, id, AccessibleEvent::..)`.
- `Key`, `Modifiers`, `Shortcut::{key, ctrl, ctrl_shift, alt, ctrl_char}`, `MouseEvent`, `PointerEvent`, `Ime::{set_enabled, is_open, set_composition_position, on_composition_changed, use_candidate_window, on_candidates_changed, candidates, select_candidate, set_candidate_page, complete, cancel, set_custom_candidates, set_custom_composition, simulate_candidates}`, `ImeCandidateWindow`, `ImeCandidates`.
- Gestures (`gestures`, in the prelude): `gestures::on_gesture(&w, \|g: &GestureEvent\| ..)?` / `on_gesture_with(&w, GestureOptions { inertia, rotation, scaling, mouse_pan, ctrl_wheel_zoom }, ..)` → `GestureSubscription::stop`; `GestureKind::{Tap, DoubleTap, Hold, Manipulation}`, `GesturePhase::{Started, Changed, Inertia, Completed}`; `GestureRecognizer` (pure logic, for tests).
- Drag and drop (`input::drag`, OLE): `start_drag(&w, &ClipboardContent, &[DropEffect])`, `start_drag_with_image(.., &PlusImage, hotspot)` → `DropEffect`; `accept_drops(&w, DropHandler::new().on_over().on_drop().on_leave())`, `stop_accepting_drops`; `DropData::{position, modifiers, allows, suggested_effect, has_text/text, has_html/html, has_image/image, has_files/files, has_format/data, format_names, from_content}`.
- Reordering by dragging: `ListBox`/`ListView`/`DataGrid` — `set_reorderable`, `on_item_reorder_request(from, to) -> bool` (`ListBox`) / `on_row_reorder_request(from, to) -> bool` (`ListView`, `DataGrid`), `on_items_reorder_request` / `on_rows_reorder_request` for blocks and `move_rows` (`DataGrid`) for several selected rows as a block, `set_drag_out_effects(&[DropEffect::Copy, DropEffect::Move])` + `on_drag_out_finished`, `on_drag_out(index) -> Option<ClipboardContent>`, `move_item`/`move_row`, `index_at`/`row_at`; `TreeView` — `set_drag_enabled`, `on_item_drop(node, TreeDropPosition) -> bool`, `on_item_moved`, `on_drag_out`, `move_item(node, TreeDropPosition::{Inside, Before, After})`, `item_at`, `TreeItem::handle`. The item follows the cursor, an insertion line in the accent color shows the target (in the tree, dropping on the middle of a node highlights it and puts the item inside), the view scrolls automatically at the edges, collapsed nodes open after ~0.7 s and `Esc` cancels.
- Library texts: `i18n::{LibraryText, library_text, library_format, set_library_text, set_library_language("en"), use_system_library_language(), system_library_language(), apply_library_translations(&Translations)}`; icon category names through `IconCategory::label()`.
- `Locale::{user_default, named}` → `format_date`, `format_long_date`, `format_time`, `format_number`, `format_currency`, `is_right_to_left`; `DateTime`; `Translations` (`key = value` catalog per language).

### Text by runs
StyledText::new(font, points), push(text, TextFormat) and to_rtf() build Unicode documents without hand-written RTF. Styles are isolated per run; it can be used in the background. Example: examples/styled_text.rs, including the native OpenFileDialog and SaveFileDialog.

Widget::invalidate() also requests painting of a container's descendants, without running callbacks synchronously.

### Composition, footer and scrolling
- StatusBarLayout: content separate from the actual bar, preferred height, limits and fit_height. The height adjustment preserves the geometry of maximized, minimized and full-screen windows; it only resizes in the normal state.
- ScrollViewport: viewport/content, automatic vertical bar, layout_content, scroll_to, scroll_wheel and ensure_visible.
- StyledText::push_mapped: styles per Unicode character with adjacent grouping.
Details in [COMPOSITION.md](COMPOSITION.md) (Portuguese); example composition.

### Visual styles by range
`RichTextBox::apply_highlights(&base, &[(range, format)]) -> GuiResult<()>` applies `TextFormat` to RichEdit UTF-16 ranges (a CRLF line break takes one position), without replacing the text, without creating undo steps and without raising edit events. It preserves the selection, the scroll position and the modified flag. It validates every range before changing the document. The application supplies the ranges and styles; no parser or language rule is built into the control. It requires RichEdit's ITextDocument and returns an explicit error when that is not available. Example: `examples/rich_highlight.rs`.

### Button content alignment
`Button::builder().content_alignment(TextAlign::Left | Center | Right)` aligns the icon + text group horizontally, keeping the 8-logical-unit inner margin and the relative position set by `icon_position`. The default stays centered. `button.set_content_alignment(..)` changes the alignment at run time; `button.content_alignment()` reads it. Long texts give way to the icon and are truncated with an ellipsis. Standalone example: `examples/button_alignment.rs`.
