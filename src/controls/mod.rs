//! Library controls.
//!
//! # Windows controls (native class wrappers)
//!
//! [`Button`], [`Label`], [`TextBox`], [`PasswordBox`], [`RichTextBox`], [`CheckBox`], [`RadioButton`],
//! [`ToggleButton`], [`ComboBox`], [`ListBox`], [`GroupBox`], [`ProgressBar`], [`Slider`], [`ScrollBar`], [`UpDown`],
//! [`DatePicker`], [`TimePicker`], [`MonthCalendar`], [`Hyperlink`], [`Tooltip`], [`StatusBar`], [`ToolBar`],
//! [`ListView`], [`TreeView`], [`TabControl`], [`HeaderControl`], [`Rebar`], [`Pager`], [`HotKeyBox`], [`IpAddressBox`]
//! and [`Animation`].
//!
//! # Own controls (composition in Rust)
//!
//! | Control | Construction |
//! |---|---|
//! | [`Panel`] | Library window class (container and custom control base). |
//! | [`SpinButton`] | Attached `EDIT` field and `UpDown`. |
//! | [`SplitContainer`] | Two [`Panel`]s and a library-drawn draggable splitter. |
//! | [`PictureBox`] | [`Panel`] drawing an icon/image scaled to available space. |
//! | [`CommandBar`] | Library-drawn [`Panel`]: buttons, menus, separators, search (hosted `EDIT`) and `»`. |
//! | [`DataGrid`] | Virtual ListView (`LVS_OWNERDATA`), Rust model and overlaid cell editor. |
//! | [`PropertyGrid`] | Two-column [`DataGrid`] with typed values and validation. |
//!
//! # Icons
//!
//! Controls displaying icons accept any [`IconSource`](crate::core::IconSource): an [`Icon`](crate::core::Icon) or a
//! bundled icon ([`crate::icons`]).
//!
//! | Control | API |
//! |---|---|
//! | [`Button`] | `builder().icon(..)`, `set_icon`, `set_icon_position` |
//! | [`Label`] | `builder().icon(..)`, `set_icon` |
//! | [`PictureBox`] | `builder().icon(..)`, `set_icon`, `set_image` |
//! | [`TreeView`] | `set_item_icon`, `set_item_icons` |
//! | [`ListView`] / [`DataGrid`] | `set_row_icon`; [`DataGrid::set_icon_provider`] for virtual sources |
//! | [`TabControl`] | `set_tab_icon` |
//! | [`ComboBox`] / [`ListBox`] | `builder().with_icons(true)` + `add_item_with_icon`, `set_item_icon` |
//! | [`ToolBar`] | `add_button(text, Some(&icon), ..)` |
//! | [`CommandBar`] | `CommandButton::new(..).icon(..)`, `CommandItem::set_icon` |
//! | [`StatusBar`] | `set_part_icon` |

pub(crate) mod common;
pub(crate) mod icon_support;

pub mod advanced;
pub mod breadcrumb;
pub mod button;
pub mod card;
pub mod chart;
pub mod checkbox;
pub mod combobox;
pub mod commandbar;
pub mod datagrid;
pub mod datetime;
pub mod dock;
pub mod expander;
pub mod infobar;
pub mod label;
pub mod listbox;
pub mod listview;
pub(crate) mod modern;
pub mod numberbox;
pub mod panel;
pub mod picturebox;
pub mod printpreview;
pub mod progressbar;
pub mod propertygrid;
pub mod rating;
pub mod richtext;
pub mod scroll_viewport;
pub mod slider;
pub mod spin;
pub mod split;
pub mod statusbar;
pub mod styled_text;
pub mod tabcontrol;
pub mod textbox;
pub mod titlebar;
pub mod toast;
pub mod toggleswitch;
pub mod toolbar;
pub mod tooltip;
pub mod treeview;

pub use advanced::{Animation, HeaderControl, HotKeyBox, Hyperlink, IpAddressBox, Pager, Rebar};
pub use button::{Button, ButtonBuilder};
pub use checkbox::{
    CheckBox, CheckBoxBuilder, CheckState, RadioButton, RadioButtonBuilder, ToggleButton, ToggleButtonBuilder,
};
pub use combobox::{ComboBox, ComboBoxBuilder};
pub use commandbar::{CommandBar, CommandBarBuilder, CommandBarLayout, CommandButton, CommandItem, COMMAND_BAR_HEIGHT};
pub use datagrid::{parse_check, CellStyle, ColumnKind, DataGrid, DataGridBuilder};
pub use datetime::{DatePicker, MonthCalendar, TimePicker};
pub use icon_support::{IconPosition, DEFAULT_ICON_SIZE};
pub use infobar::{InfoBar, InfoBarBuilder, Severity, INFO_BAR_HEIGHT};
pub use toast::{Toast, ToastHandle};

/// Default logical height (96 DPI) of [`Button`] and [`ComboBox`] for automatic side-by-side alignment.
pub const DEFAULT_CONTROL_HEIGHT: i32 = 28;
pub use breadcrumb::{BreadcrumbBar, BreadcrumbBarBuilder};
pub use card::{Card, CardBuilder};
pub use chart::{Chart, ChartBuilder, ChartHit, ChartKind, ChartSeries, CHART_PALETTE};
pub use dock::{DockHost, DockHostBuilder, DockPane, DockSide};
pub use expander::{Expander, ExpanderBuilder, EXPANDER_HEADER};
pub use label::{Label, LabelBuilder};
pub use listbox::{ListBox, ListBoxBuilder};
pub use listview::{ColumnAlign, ListView, ListViewBuilder};
pub use numberbox::{NumberBox, NumberBoxBuilder};
pub use panel::{GroupBox, GroupBoxBuilder, Panel, PanelBuilder};
pub use picturebox::{PictureBox, PictureBoxBuilder};
pub use printpreview::{PreviewZoom, PrintPreview, PrintPreviewBuilder, PrintPreviewDialog, PrintPreviewWindow};
pub use progressbar::{ProgressBar, ProgressBarBuilder, ProgressState};
pub use propertygrid::{ChoiceMode, PropertyGrid, PropertyValue};
pub use rating::{RatingControl, RatingControlBuilder};
pub use richtext::{ListStyle, RichTextBox, RichTextBoxBuilder, TextFormat};
pub use scroll_viewport::ScrollViewport;
pub use slider::{ScrollBar, ScrollBarBuilder, Slider, SliderBuilder};
pub use spin::{SpinButton, UpDown};
pub use split::SplitContainer;
pub use statusbar::StatusBar;
pub use styled_text::StyledText;
pub use tabcontrol::TabControl;
pub use textbox::{PasswordBox, PasswordBoxBuilder, TextBox, TextBoxBuilder};
pub use titlebar::{CaptionButton, TitleBar, TITLE_BAR_HEIGHT};
pub use toggleswitch::{ToggleSwitch, ToggleSwitchBuilder};
pub use toolbar::ToolBar;
pub use tooltip::Tooltip;
pub use treeview::{TreeDropPosition, TreeItem, TreeView};
mod rich_highlight;
