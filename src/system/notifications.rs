//! Windows 10/11 Action Center notifications with buttons, images and click callbacks, using handwritten Windows
//! Runtime calls (`Windows.UI.Notifications`), no external libraries.
//!
//! ```no_run
//! # use arqrustui::prelude::*;
//! # fn demo() -> GuiResult<()> {
//! SystemNotification::new("Backup complete")
//!     .body("1,248 files copied to D:\\Backup.")
//!     .button("Open folder", "open")
//!     .button("Close", "close")
//!     .on_activated(|argument| println!("clicked: {argument}"))
//!     .show()?;
//! # Ok(()) }
//! ```
//!
//! Requires *AppUserModelID*. On first notification registers an executable-derived ID in
//! `HKCU\Software\Classes\AppUserModelId`, without installer/Start shortcut. Call [`SystemNotification::set_app_id`]
//! first for custom ID, display name and icon.
//!
//! While running, clicks reach [`on_activated`](SystemNotification::on_activated). To **reopen after exiting**,
//! register a COM [`NotificationActivator::register`] at `main` startup; records COM server/CustomActivator in HKCU.
//!
//! Supports [`text_input`](SystemNotification::text_input), [`selection_input`](SystemNotification::selection_input),
//! updateable progress ([`progress`](SystemNotification::progress), [`SystemNotificationHandle::update_progress`]),
//! scheduling ([`show_at`](SystemNotification::show_at)), [`tag`](SystemNotification::tag),
//! [`group`](SystemNotification::group), [`header`](SystemNotification::header) and
//! [`SystemNotification::remove_group`].

use crate::core::application as app;
use crate::core::{GuiError, GuiResult, UiDispatcher};
use crate::ffi::ole32::{ComPtr, IID_IUnknown};
use crate::ffi::winrt::*;
use crate::ffi::*;
use crate::utilities::to_wide;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

mod activator;
mod winrt;

pub use activator::NotificationActivator;
use winrt::*;

/// Notification click: button/body argument and entered/selected fields (`id` → value).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct NotificationActivation {
    /// Clicked button argument or [`launch_argument`](SystemNotification::launch_argument).
    pub argument: String,
    /// Text/selection input values by identifier.
    pub inputs: Vec<(String, String)>,
}

impl NotificationActivation {
    /// Field `id` value, if any.
    pub fn input(&self, id: &str) -> Option<&str> {
        self.inputs.iter().find(|(key, _)| key == id).map(|(_, value)| value.as_str())
    }
}

/// Notification input field.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Input {
    Text { id: String, placeholder: String, title: String },
    Selection { id: String, title: String, options: Vec<(String, String)>, default: Option<String> },
}

/// Notification progress bar.
#[derive(Clone, Debug, PartialEq)]
struct Progress {
    title: String,
    /// `None` = indeterminate.
    value: Option<f64>,
    value_text: String,
    status: String,
}

/// Scenario controlling on-screen behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum NotificationScenario {
    /// Normal behavior.
    #[default]
    Default,
    /// Reminder: remains until user action.
    Reminder,
    /// Alarm: remains and repeats sound.
    Alarm,
    /// Incoming call, fullscreen on some devices.
    IncomingCall,
    /// Urgent: bypasses Do Not Disturb (Windows 10 build 22546+).
    Urgent,
}

/// Converts time to `Windows.Foundation.DateTime` (100 ns since 1601-01-01 UTC).
pub(crate) fn to_winrt_time(when: SystemTime) -> i64 {
    const UNIX_TO_1601: i64 = 11_644_473_600;
    match when.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(after) => (after.as_secs() as i64 + UNIX_TO_1601) * 10_000_000 + after.subsec_nanos() as i64 / 100,
        Err(before) => {
            let before = before.duration();
            (UNIX_TO_1601 - before.as_secs() as i64) * 10_000_000 - before.subsec_nanos() as i64 / 100
        }
    }
}

/// Why notification left screen without a click.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DismissReason {
    /// User dismissed.
    UserCanceled,
    /// Program hid it via [`SystemNotificationHandle::hide`].
    ApplicationHidden,
    /// Display timeout, still in Action Center.
    TimedOut,
}

type ActivatedFn = Rc<dyn Fn(&NotificationActivation)>;
type DismissedFn = Rc<dyn Fn(DismissReason)>;
/// Displayed notification callbacks until first event.
type Callbacks = (Option<ActivatedFn>, Option<DismissedFn>);

/// Action Center notification; build then [`show`](Self::show).
#[derive(Clone, Default)]
pub struct SystemNotification {
    title: String,
    lines: Vec<String>,
    attribution: Option<String>,
    image: Option<PathBuf>,
    hero: Option<PathBuf>,
    buttons: Vec<(String, String)>,
    launch: String,
    silent: bool,
    long_duration: bool,
    on_activated: Option<ActivatedFn>,
    on_dismissed: Option<DismissedFn>,
    inputs: Vec<Input>,
    /// Button index → adjacent text input field.
    button_inputs: Vec<(usize, String)>,
    progress: Option<Progress>,
    tag: Option<String>,
    group: Option<String>,
    header: Option<(String, String, String)>,
    scenario: NotificationScenario,
    suppress_popup: bool,
}

impl std::fmt::Debug for SystemNotification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemNotification").field("title", &self.title).finish_non_exhaustive()
    }
}

/// Application ID, display name and icon.
struct AppIdentity {
    id: String,
    registered: bool,
}

static APP_ID: Mutex<Option<AppIdentity>> = Mutex::new(None);
static NEXT_KEY: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static HANDLERS: RefCell<HashMap<u64, Callbacks>> = RefCell::new(HashMap::new());
}

/// XML-escaped text.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if (c as u32) < 0x20 && c != '\n' && c != '\t' => {}
            c => out.push(c),
        }
    }
    out
}

/// Absolute path as `file:///` URI.
fn file_uri(path: &Path) -> String {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let text = absolute.to_string_lossy().replace('\\', "/");
    format!("file:///{}", text.trim_start_matches('/'))
}

impl SystemNotification {
    /// Notification with given title.
    pub fn new(title: &str) -> SystemNotification {
        SystemNotification { title: title.to_string(), ..SystemNotification::default() }
    }

    /// Adds a text line; Windows shows up to two beyond title.
    pub fn body(mut self, text: &str) -> SystemNotification {
        self.lines.push(text.to_string());
        self
    }

    /// Small source text below body, e.g. "via Email".
    pub fn attribution(mut self, text: &str) -> SystemNotification {
        self.attribution = Some(text.to_string());
        self
    }

    /// Small image left of text replacing app icon.
    pub fn image(mut self, path: impl AsRef<Path>) -> SystemNotification {
        self.image = Some(path.as_ref().to_path_buf());
        self
    }

    /// Large image at top.
    pub fn hero_image(mut self, path: impl AsRef<Path>) -> SystemNotification {
        self.hero = Some(path.as_ref().to_path_buf());
        self
    }

    /// Adds up to five buttons. `argument` reaches [`on_activated`](Self::on_activated) on click.
    pub fn button(mut self, label: &str, argument: &str) -> SystemNotification {
        self.buttons.push((label.to_string(), argument.to_string()));
        self
    }

    /// Body click argument for [`on_activated`](Self::on_activated), default empty.
    pub fn launch_argument(mut self, argument: &str) -> SystemNotification {
        self.launch = argument.to_string();
        self
    }

    /// Silent.
    pub fn silent(mut self) -> SystemNotification {
        self.silent = true;
        self
    }

    /// Displays about 25 s rather than ~7 s.
    pub fn long_duration(mut self) -> SystemNotification {
        self.long_duration = true;
        self
    }

    /// Body/button click receives corresponding argument on UI thread.
    pub fn on_activated(mut self, f: impl Fn(&str) + 'static) -> SystemNotification {
        self.on_activated = Some(Rc::new(move |activation: &NotificationActivation| f(&activation.argument)));
        self
    }

    /// Like [`on_activated`](Self::on_activated), including text/selection values.
    pub fn on_activated_with(mut self, f: impl Fn(&NotificationActivation) + 'static) -> SystemNotification {
        self.on_activated = Some(Rc::new(f));
        self
    }

    /// Text input, up to five fields; value in [`NotificationActivation::input`] by `id`.
    pub fn text_input(mut self, id: &str, placeholder: &str) -> SystemNotification {
        self.inputs.push(Input::Text {
            id: id.to_string(),
            placeholder: placeholder.to_string(),
            title: String::new(),
        });
        self
    }

    /// Button beside `input_id`, e.g. quick-reply Send.
    pub fn button_for_input(mut self, label: &str, argument: &str, input_id: &str) -> SystemNotification {
        self.button_inputs.push((self.buttons.len(), input_id.to_string()));
        self.buttons.push((label.to_string(), argument.to_string()));
        self
    }

    /// Options `(id, text)` with initial option `default`.
    pub fn selection_input(
        mut self,
        id: &str,
        title: &str,
        options: &[(&str, &str)],
        default: Option<&str>,
    ) -> SystemNotification {
        self.inputs.push(Input::Selection {
            id: id.to_string(),
            title: title.to_string(),
            options: options.iter().map(|(i, c)| (i.to_string(), c.to_string())).collect(),
            default: default.map(str::to_string),
        });
        self
    }

    /// Progress: `value` 0 to 1 (`None` indeterminate), `value_text` replaces percentage ("3/10 files"), `status` below
    /// ("Copying..."). Update with [`SystemNotificationHandle::update_progress`] on Windows 10 1703+.
    pub fn progress(mut self, title: &str, value: Option<f64>, value_text: &str, status: &str) -> SystemNotification {
        self.progress = Some(Progress {
            title: title.to_string(),
            value: value.map(|v| v.clamp(0.0, 1.0)),
            value_text: value_text.to_string(),
            status: status.to_string(),
        });
        self
    }

    /// Tag: new notification with same tag/group replaces previous.
    pub fn tag(mut self, tag: &str) -> SystemNotification {
        self.tag = Some(tag.chars().take(64).collect());
        self
    }

    /// Group for bulk [`SystemNotification::remove_group`].
    pub fn group(mut self, group: &str) -> SystemNotification {
        self.group = Some(group.chars().take(64).collect());
        self
    }

    /// Header visually grouping same-`id` notifications in Action Center (Windows 10 1709+); click delivers `argument`
    /// to `on_activated`.
    pub fn header(mut self, id: &str, title: &str, argument: &str) -> SystemNotification {
        self.header = Some((id.to_string(), title.to_string(), argument.to_string()));
        self
    }

    /// Scenario: reminder, alarm, call, urgent.
    pub fn scenario(mut self, scenario: NotificationScenario) -> SystemNotification {
        self.scenario = scenario;
        self
    }

    /// Goes directly to Action Center without appearing on screen.
    pub fn suppress_popup(mut self) -> SystemNotification {
        self.suppress_popup = true;
        self
    }

    /// Dismissed without click, on UI thread.
    pub fn on_dismissed(mut self, f: impl Fn(DismissReason) + 'static) -> SystemNotification {
        self.on_dismissed = Some(Rc::new(f));
        self
    }

    /// Notification XML (`ToastGeneric` schema).
    pub fn to_xml(&self) -> String {
        let mut xml = format!("<toast launch=\"{}\"", escape(&self.launch));
        if self.long_duration {
            xml.push_str(" duration=\"long\"");
        }
        match self.scenario {
            NotificationScenario::Default => {}
            NotificationScenario::Reminder => xml.push_str(" scenario=\"reminder\""),
            NotificationScenario::Alarm => xml.push_str(" scenario=\"alarm\""),
            NotificationScenario::IncomingCall => xml.push_str(" scenario=\"incomingCall\""),
            NotificationScenario::Urgent => xml.push_str(" scenario=\"urgent\""),
        }
        xml.push('>');
        if let Some((id, title, argument)) = &self.header {
            xml.push_str(&format!(
                "<header id=\"{}\" title=\"{}\" arguments=\"{}\"/>",
                escape(id),
                escape(title),
                escape(argument)
            ));
        }
        xml.push_str("<visual><binding template=\"ToastGeneric\">");
        xml.push_str(&format!("<text>{}</text>", escape(&self.title)));
        for line in self.lines.iter().take(2) {
            xml.push_str(&format!("<text>{}</text>", escape(line)));
        }
        if let Some(text) = &self.attribution {
            xml.push_str(&format!("<text placement=\"attribution\">{}</text>", escape(text)));
        }
        if let Some(path) = &self.image {
            xml.push_str(&format!("<image placement=\"appLogoOverride\" src=\"{}\"/>", escape(&file_uri(path))));
        }
        if let Some(path) = &self.hero {
            xml.push_str(&format!("<image placement=\"hero\" src=\"{}\"/>", escape(&file_uri(path))));
        }
        if self.progress.is_some() {
            // Valores ligados a `NotificationData` (atualizáveis depois).
            xml.push_str(
                "<progress title=\"{progressTitle}\" value=\"{progressValue}\" \
                 valueStringOverride=\"{progressValueString}\" status=\"{progressStatus}\"/>",
            );
        }
        xml.push_str("</binding></visual>");
        if !self.buttons.is_empty() || !self.inputs.is_empty() {
            xml.push_str("<actions>");
            for input in self.inputs.iter().take(5) {
                match input {
                    Input::Text { id, placeholder, title } => {
                        xml.push_str(&format!(
                            "<input id=\"{}\" type=\"text\" placeHolderContent=\"{}\"",
                            escape(id),
                            escape(placeholder)
                        ));
                        if !title.is_empty() {
                            xml.push_str(&format!(" title=\"{}\"", escape(title)));
                        }
                        xml.push_str("/>");
                    }
                    Input::Selection { id, title, options, default } => {
                        xml.push_str(&format!("<input id=\"{}\" type=\"selection\"", escape(id)));
                        if !title.is_empty() {
                            xml.push_str(&format!(" title=\"{}\"", escape(title)));
                        }
                        if let Some(default) = default {
                            xml.push_str(&format!(" defaultInput=\"{}\"", escape(default)));
                        }
                        xml.push('>');
                        for (option, content) in options.iter().take(5) {
                            xml.push_str(&format!(
                                "<selection id=\"{}\" content=\"{}\"/>",
                                escape(option),
                                escape(content)
                            ));
                        }
                        xml.push_str("</input>");
                    }
                }
            }
            for (index, (label, argument)) in self.buttons.iter().take(5).enumerate() {
                xml.push_str(&format!(
                    "<action content=\"{}\" arguments=\"{}\" activationType=\"foreground\"",
                    escape(label),
                    escape(argument)
                ));
                if let Some((_, input)) = self.button_inputs.iter().find(|(i, _)| *i == index) {
                    xml.push_str(&format!(" hint-inputId=\"{}\"", escape(input)));
                }
                xml.push_str("/>");
            }
            xml.push_str("</actions>");
        }
        if self.silent {
            xml.push_str("<audio silent=\"true\"/>");
        }
        xml.push_str("</toast>");
        xml
    }

    /// Whether Windows Runtime notifications exist (Windows 8+).
    pub fn is_supported() -> bool {
        winrt().is_some_and(|apis| {
            HString::new(apis, "Windows.UI.Notifications.ToastNotificationManager")
                .and_then(|name| activation_factory(apis, &name, &IID_IToastNotificationManagerStatics))
                .is_ok()
        })
    }

    /// Chooses AppUserModelID (e.g. `"MyCompany.MyApp"`), display name and optional ICO/PNG icon.
    ///
    /// Registers in `HKCU\Software\Classes\AppUserModelId` and associates with process. Call before first notification.
    pub fn set_app_id(id: &str, display_name: &str, icon: Option<&Path>) -> GuiResult<()> {
        if id.is_empty() || id.len() > 128 || id.contains(char::is_whitespace) {
            return Err(GuiError::InvalidArgument(
                "AppUserModelID must be at most 128 characters, without spaces".into(),
            ));
        }
        register_app_id(id, display_name, icon)?;
        *APP_ID.lock().unwrap_or_else(|p| p.into_inner()) = Some(AppIdentity { id: id.to_string(), registered: true });
        Ok(())
    }

    /// Current explicit/executable-derived ID.
    pub fn app_id() -> String {
        let mut guard = APP_ID.lock().unwrap_or_else(|p| p.into_inner());
        guard.get_or_insert_with(|| AppIdentity { id: default_app_id(), registered: false }).id.clone()
    }

    /// XML loaded into WinRT XmlDocument.
    ///
    /// # Safety
    /// Requires available WinRT (`apis`).
    unsafe fn document(&self, apis: &'static WinRtApis) -> GuiResult<ComPtr> {
        let class = HString::new(apis, "Windows.Data.Xml.Dom.XmlDocument")?;
        let mut raw: *mut c_void = std::ptr::null_mut();
        check("RoActivateInstance(XmlDocument)", (apis.RoActivateInstance)(class.0, &mut raw))?;
        let document = ComPtr::from_raw(raw).ok_or(GuiError::Unsupported("XmlDocument"))?;
        let io = query(&document, &IID_IXmlDocumentIO)?;
        let xml = HString::new(apis, &self.to_xml())?;
        let load: unsafe extern "system" fn(*mut c_void, HSTRING) -> HRESULT = io.method(6);
        check("XmlDocument::LoadXml", load(io.as_ptr(), xml.0))?;
        query(&document, &IID_IXmlDocument)
    }

    /// Effective tag, generated if absent for progress notifications.
    fn effective_tag(&self, key: u64) -> String {
        match (&self.tag, &self.progress) {
            (Some(tag), _) => tag.clone(),
            (None, Some(_)) => format!("arqrustui-{key}"),
            (None, None) => String::new(),
        }
    }

    /// App notifier (`CreateToastNotifierWithId`).
    ///
    /// # Safety
    /// Requires available WinRT (`apis`).
    unsafe fn notifier(apis: &'static WinRtApis, id: &str) -> GuiResult<ComPtr> {
        let class = HString::new(apis, "Windows.UI.Notifications.ToastNotificationManager")?;
        let statics = activation_factory(apis, &class, &IID_IToastNotificationManagerStatics)?;
        let create_notifier: unsafe extern "system" fn(*mut c_void, HSTRING, *mut *mut c_void) -> HRESULT =
            statics.method(7);
        let app_id = HString::new(apis, id)?;
        let mut raw: *mut c_void = std::ptr::null_mut();
        check("CreateToastNotifierWithId", create_notifier(statics.as_ptr(), app_id.0, &mut raw))?;
        ComPtr::from_raw(raw).ok_or(GuiError::Unsupported("ToastNotifier"))
    }

    /// Shows notification; [`GuiError::Unsupported`] without WinRT notifications (Windows 7/Wine).
    pub fn show(&self) -> GuiResult<SystemNotificationHandle> {
        let apis = winrt().ok_or(GuiError::Unsupported("system notifications require Windows 8 or later"))?;
        let dispatcher = UiDispatcher::new(app::require()?.shared.clone());
        let id = ensure_app_id()?;
        let key = NEXT_KEY.fetch_add(1, Ordering::Relaxed);
        let tag = self.effective_tag(key);
        let group = self.group.clone().unwrap_or_default();
        // SAFETY: chamadas WinRT pelos índices de vtable documentados (IInspectable tem 6
        // métodos; os próprios começam no índice 6); cada referência obtida passa a um
        // ComPtr, e as HSTRINGs são liberadas pelo HString.
        unsafe {
            let document = self.document(apis)?;

            // ToastNotification.
            let class = HString::new(apis, "Windows.UI.Notifications.ToastNotification")?;
            let factory = activation_factory(apis, &class, &IID_IToastNotificationFactory)?;
            let create: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut *mut c_void) -> HRESULT =
                factory.method(6);
            let mut raw: *mut c_void = std::ptr::null_mut();
            check("CreateToastNotification", create(factory.as_ptr(), document.as_ptr(), &mut raw))?;
            let toast = ComPtr::from_raw(raw).ok_or(GuiError::Unsupported("ToastNotification"))?;

            // Etiqueta, grupo e "sem popup" (IToastNotification2, Windows 10).
            if !tag.is_empty() || !group.is_empty() || self.suppress_popup {
                let toast2 = query(&toast, &IID_IToastNotification2)?;
                type PutText = unsafe extern "system" fn(*mut c_void, HSTRING) -> HRESULT;
                if !tag.is_empty() {
                    let text = HString::new(apis, &tag)?;
                    check("ToastNotification::put_Tag", toast2.method::<PutText>(6)(toast2.as_ptr(), text.0))?;
                }
                if !group.is_empty() {
                    let text = HString::new(apis, &group)?;
                    check("ToastNotification::put_Group", toast2.method::<PutText>(8)(toast2.as_ptr(), text.0))?;
                }
                if self.suppress_popup {
                    let put: unsafe extern "system" fn(*mut c_void, u8) -> HRESULT = toast2.method(10);
                    put(toast2.as_ptr(), 1);
                }
            }
            // Valores iniciais da barra de progresso (IToastNotification4, Windows 10 1703+).
            if let Some(progress) = &self.progress {
                let data = notification_data(apis, &progress_values(progress, true), 1)?;
                let toast4 = query(&toast, &IID_IToastNotification4)?;
                let put: unsafe extern "system" fn(*mut c_void, *mut c_void) -> HRESULT = toast4.method(7);
                check("ToastNotification::put_Data", put(toast4.as_ptr(), data.as_ptr()))?;
            }

            // Eventos: um objeto delegado por evento, que repassa à thread da interface.
            HANDLERS.with(|h| h.borrow_mut().insert(key, (self.on_activated.clone(), self.on_dismissed.clone())));
            let input_ids: Vec<String> = self
                .inputs
                .iter()
                .map(|input| match input {
                    Input::Text { id, .. } | Input::Selection { id, .. } => id.clone(),
                })
                .collect();
            for (index, kind) in [(11, Kind::Activated), (9, Kind::Dismissed), (13, Kind::Failed)] {
                let handler = Handler::create(kind, key, dispatcher.clone(), input_ids.clone());
                let add: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut i64) -> HRESULT = toast.method(index);
                let mut token = 0i64;
                add(toast.as_ptr(), handler.as_ptr(), &mut token);
            }

            let notifier = Self::notifier(apis, &id)?;
            let show: unsafe extern "system" fn(*mut c_void, *mut c_void) -> HRESULT = notifier.method(6);
            if let Err(error) = check("ToastNotifier::Show", show(notifier.as_ptr(), toast.as_ptr())) {
                HANDLERS.with(|h| h.borrow_mut().remove(&key));
                return Err(error);
            }
            Ok(SystemNotificationHandle { notifier, toast, tag, group, sequence: std::cell::Cell::new(1) })
        }
    }

    /// Schedules at `when`, displayed by Windows even after exit. Buttons/fields work; `on_activated` receives clicks
    /// only while running or with [`NotificationActivator`].
    pub fn show_at(&self, when: SystemTime) -> GuiResult<ScheduledNotification> {
        let apis = winrt().ok_or(GuiError::Unsupported("system notifications require Windows 8 or later"))?;
        if when <= SystemTime::now() {
            return Err(GuiError::InvalidArgument("the scheduled time must be in the future".into()));
        }
        let id = ensure_app_id()?;
        let key = NEXT_KEY.fetch_add(1, Ordering::Relaxed);
        let tag = self.effective_tag(key);
        // SAFETY: chamadas WinRT pelos índices documentados; referências em ComPtr.
        unsafe {
            let document = self.document(apis)?;
            let class = HString::new(apis, "Windows.UI.Notifications.ScheduledToastNotification")?;
            let factory = activation_factory(apis, &class, &IID_IScheduledToastNotificationFactory)?;
            let create: unsafe extern "system" fn(*mut c_void, *mut c_void, i64, *mut *mut c_void) -> HRESULT =
                factory.method(6);
            let mut raw: *mut c_void = std::ptr::null_mut();
            check(
                "CreateScheduledToastNotification",
                create(factory.as_ptr(), document.as_ptr(), to_winrt_time(when), &mut raw),
            )?;
            let scheduled = ComPtr::from_raw(raw).ok_or(GuiError::Unsupported("ScheduledToastNotification"))?;
            if !tag.is_empty() || self.group.is_some() {
                let scheduled2 = query(&scheduled, &IID_IScheduledToastNotification2)?;
                type PutText = unsafe extern "system" fn(*mut c_void, HSTRING) -> HRESULT;
                if !tag.is_empty() {
                    let text = HString::new(apis, &tag)?;
                    scheduled2.method::<PutText>(6)(scheduled2.as_ptr(), text.0);
                }
                if let Some(group) = &self.group {
                    let text = HString::new(apis, group)?;
                    scheduled2.method::<PutText>(8)(scheduled2.as_ptr(), text.0);
                }
            }
            let notifier = Self::notifier(apis, &id)?;
            let add: unsafe extern "system" fn(*mut c_void, *mut c_void) -> HRESULT = notifier.method(9);
            check("ToastNotifier::AddToSchedule", add(notifier.as_ptr(), scheduled.as_ptr()))?;
            Ok(ScheduledNotification { notifier, scheduled, when })
        }
    }

    /// Removes Action Center notification by tag/group.
    pub fn remove(tag: &str, group: Option<&str>) -> GuiResult<()> {
        with_history(|apis, history, app| {
            let tag = HString::new(apis, tag)?;
            // SAFETY: índices de IToastNotificationHistory (RemoveGroupedTagWithId = 8,
            // Remove = 10, com HSTRINGs vivas).
            unsafe {
                match group {
                    Some(group) => {
                        let group = HString::new(apis, group)?;
                        let remove: unsafe extern "system" fn(*mut c_void, HSTRING, HSTRING, HSTRING) -> HRESULT =
                            history.method(8);
                        check("ToastNotificationHistory::Remove", remove(history.as_ptr(), tag.0, group.0, app.0))
                    }
                    None => {
                        let remove: unsafe extern "system" fn(*mut c_void, HSTRING) -> HRESULT = history.method(10);
                        check("ToastNotificationHistory::Remove", remove(history.as_ptr(), tag.0))
                    }
                }
            }
        })
    }

    /// Removes all Action Center notifications in group.
    pub fn remove_group(group: &str) -> GuiResult<()> {
        with_history(|apis, history, app| {
            let group = HString::new(apis, group)?;
            // SAFETY: RemoveGroupWithId (índice 7) com HSTRINGs vivas.
            unsafe {
                let remove: unsafe extern "system" fn(*mut c_void, HSTRING, HSTRING) -> HRESULT = history.method(7);
                check("ToastNotificationHistory::RemoveGroup", remove(history.as_ptr(), group.0, app.0))
            }
        })
    }

    /// Removes all app notifications from Action Center.
    pub fn clear_all() -> GuiResult<()> {
        with_history(|_, history, app| {
            // SAFETY: ClearWithId (índice 12) com a HSTRING do aplicativo viva.
            unsafe {
                let clear: unsafe extern "system" fn(*mut c_void, HSTRING) -> HRESULT = history.method(12);
                check("ToastNotificationHistory::Clear", clear(history.as_ptr(), app.0))
            }
        })
    }

    /// Whether Windows launched the program for a notification click using registered [`NotificationActivator`].
    pub fn launched_by_notification() -> bool {
        std::env::args().skip(1).any(|a| a.eq_ignore_ascii_case(activator::LAUNCH_SWITCH) || a == "-Embedding")
    }
}

/// Displayed notification handle for hiding/progress updates.
pub struct SystemNotificationHandle {
    notifier: ComPtr,
    toast: ComPtr,
    tag: String,
    group: String,
    sequence: std::cell::Cell<u32>,
}

impl std::fmt::Debug for SystemNotificationHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemNotificationHandle").finish_non_exhaustive()
    }
}

impl SystemNotificationHandle {
    /// Updates progress 0 to 1 (`None` indeterminate) without redisplay. Returns `false` if dismissed. Requires Windows
    /// 10 1703+.
    pub fn update_progress(&self, value: Option<f64>, value_text: &str, status: &str) -> GuiResult<bool> {
        let apis = winrt().ok_or(GuiError::Unsupported("system notifications"))?;
        let sequence = self.sequence.get() + 1;
        self.sequence.set(sequence);
        let progress = Progress {
            title: String::new(),
            value: value.map(|v| v.clamp(0.0, 1.0)),
            value_text: value_text.to_string(),
            status: status.to_string(),
        };
        // SAFETY: chamadas WinRT pelos índices documentados; referências em ComPtr.
        unsafe {
            let data = notification_data(apis, &progress_values(&progress, false), sequence)?;
            let notifier2 = query(&self.notifier, &IID_IToastNotifier2)?;
            let update: unsafe extern "system" fn(*mut c_void, *mut c_void, HSTRING, HSTRING, *mut i32) -> HRESULT =
                notifier2.method(6);
            let tag = HString::new(apis, &self.tag)?;
            let group = HString::new(apis, &self.group)?;
            let mut result = 0i32;
            check("ToastNotifier::Update", update(notifier2.as_ptr(), data.as_ptr(), tag.0, group.0, &mut result))?;
            // NotificationUpdateResult: 0 = Succeeded, 1 = Failed, 2 = NotificationNotFound.
            Ok(result == 0)
        }
    }

    /// Explicit/generated progress tag.
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// Removes from screen/Action Center.
    pub fn hide(&self) -> GuiResult<()> {
        // SAFETY: índice 7 de IToastNotifier = Hide(IToastNotification*).
        unsafe {
            let hide: unsafe extern "system" fn(*mut c_void, *mut c_void) -> HRESULT = self.notifier.method(7);
            check("ToastNotifier::Hide", hide(self.notifier.as_ptr(), self.toast.as_ptr()))
        }
    }
}

/// Scheduled [`SystemNotification::show_at`] handle for cancellation.
pub struct ScheduledNotification {
    notifier: ComPtr,
    scheduled: ComPtr,
    when: SystemTime,
}

impl std::fmt::Debug for ScheduledNotification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScheduledNotification").field("when", &self.when).finish_non_exhaustive()
    }
}

impl ScheduledNotification {
    /// Scheduled time.
    pub fn when(&self) -> SystemTime {
        self.when
    }

    /// Cancels scheduling, no effect after display.
    pub fn cancel(&self) -> GuiResult<()> {
        // SAFETY: RemoveFromSchedule (índice 10 de IToastNotifier) com objetos vivos.
        unsafe {
            let remove: unsafe extern "system" fn(*mut c_void, *mut c_void) -> HRESULT = self.notifier.method(10);
            check("ToastNotifier::RemoveFromSchedule", remove(self.notifier.as_ptr(), self.scheduled.as_ptr()))
        }
    }
}

/// Simulates a Windows event for tests.
#[doc(hidden)]
pub fn debug_deliver_latest(argument: Option<&str>, reason: Option<DismissReason>) -> bool {
    let key = HANDLERS.with(|h| h.borrow().keys().max().copied());
    let Some(key) = key else { return false };
    match (argument, reason) {
        (Some(argument), _) => deliver(
            key,
            Fired::Activated(NotificationActivation { argument: argument.to_string(), inputs: Vec::new() }),
        ),
        (None, Some(reason)) => deliver(key, Fired::Dismissed(reason)),
        _ => deliver(key, Fired::Failed),
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_escapes_text_and_lists_buttons() {
        let xml = SystemNotification::new("Olá <mundo> & \"cia\"")
            .body("linha 1")
            .body("linha 2")
            .body("linha 3 (ignorada)")
            .attribution("via teste")
            .button("Abrir", "abrir&1")
            .launch_argument("corpo")
            .silent()
            .long_duration()
            .to_xml();
        assert!(
            xml.starts_with("<toast launch=\"corpo\" duration=\"long\"><visual><binding template=\"ToastGeneric\">")
        );
        assert!(xml.contains("<text>Olá &lt;mundo&gt; &amp; &quot;cia&quot;</text>"));
        assert!(xml.contains("<text>linha 2</text>") && !xml.contains("linha 3"));
        assert!(xml.contains("<text placement=\"attribution\">via teste</text>"));
        assert!(xml.contains("<action content=\"Abrir\" arguments=\"abrir&amp;1\" activationType=\"foreground\"/>"));
        assert!(xml.ends_with("<audio silent=\"true\"/></toast>"));
    }

    #[test]
    fn images_become_file_uris() {
        let xml = SystemNotification::new("x").image("C:\\imagens\\logo.png").to_xml();
        assert!(xml.contains("src=\"file:///C:/imagens/logo.png\""), "{xml}");
    }

    #[test]
    fn default_app_id_is_a_valid_identifier() {
        let id = default_app_id();
        assert!(id.starts_with("ArqRustUI.") && !id.contains(' '), "{id}");
    }

    /// Delegate IIDs follow WinRT parameterized-type rules (signature SHA-1), verified against an implementation here.
    #[test]
    fn delegate_iids_follow_the_parameterized_type_rule() {
        let typed = "{9de1c534-6ae1-11e0-84e1-18a905bcc53f}";
        let toast = "rc(Windows.UI.Notifications.ToastNotification;{997e2675-059e-4e60-8b06-1760917c8b80})";
        let cases = [
            (format!("pinterface({typed};{toast};cinterface(IInspectable))"), IID_ActivatedHandler),
            (
                format!("pinterface({typed};{toast};rc(Windows.UI.Notifications.ToastDismissedEventArgs;{{3f89d935-d9cb-4538-a0f0-ffe7659938f8}}))"),
                IID_DismissedHandler,
            ),
            (
                format!("pinterface({typed};{toast};rc(Windows.UI.Notifications.ToastFailedEventArgs;{{35176862-cfd4-44f8-ad64-f500fd896c3b}}))"),
                IID_FailedHandler,
            ),
        ];
        for (signature, expected) in cases {
            assert_eq!(parameterized_iid(&signature), expected, "{signature}");
        }
        let map = "{3c2925fe-8519-45c1-aa79-197b6718c1c1}";
        assert_eq!(parameterized_iid(&format!("pinterface({map};string;string)")), IID_IMap_String_String);
        assert_eq!(
            parameterized_iid(&format!("pinterface({map};string;cinterface(IInspectable))")),
            IID_IMap_String_Object
        );
    }

    #[test]
    fn xml_has_inputs_progress_header_and_scenario() {
        let xml = SystemNotification::new("Mensagem")
            .header("conversa-7", "Equipe", "abrir-conversa")
            .scenario(NotificationScenario::Reminder)
            .text_input("resposta", "Digite <aqui>")
            .button_for_input("Enviar", "enviar", "resposta")
            .selection_input("prazo", "Lembrar em", &[("5", "5 min"), ("60", "1 h")], Some("60"))
            .progress("Cópia", Some(0.25), "1/4", "Copiando...")
            .to_xml();
        assert!(xml.starts_with("<toast launch=\"\" scenario=\"reminder\"><header id=\"conversa-7\" title=\"Equipe\" arguments=\"abrir-conversa\"/>"));
        assert!(xml.contains("<progress title=\"{progressTitle}\" value=\"{progressValue}\""));
        assert!(xml.contains("<input id=\"resposta\" type=\"text\" placeHolderContent=\"Digite &lt;aqui&gt;\"/>"));
        assert!(xml.contains("<input id=\"prazo\" type=\"selection\" title=\"Lembrar em\" defaultInput=\"60\"><selection id=\"5\" content=\"5 min\"/>"));
        assert!(xml.contains(
            "<action content=\"Enviar\" arguments=\"enviar\" activationType=\"foreground\" hint-inputId=\"resposta\"/>"
        ));
        let progress = Progress { title: "T".into(), value: Some(0.5), value_text: "x".into(), status: "s".into() };
        let values = progress_values(&progress, true);
        assert_eq!(values[0], ("progressValue", "0.5".to_string()));
        assert_eq!(values.len(), 4);
        let unknown = Progress { value: None, ..progress };
        assert_eq!(progress_values(&unknown, false)[0].1, "indeterminate");
        let activation =
            NotificationActivation { argument: "a".into(), inputs: vec![("resposta".into(), "oi".into())] };
        assert_eq!((activation.input("resposta"), activation.input("x")), (Some("oi"), None));
    }

    #[test]
    fn winrt_time_counts_from_1601() {
        assert_eq!(to_winrt_time(SystemTime::UNIX_EPOCH), 116_444_736_000_000_000);
        let later = SystemTime::UNIX_EPOCH + std::time::Duration::from_millis(1500);
        assert_eq!(to_winrt_time(later), 116_444_736_015_000_000);
    }

    /// Parameterized IID: SHA-1(namespace || UTF-8 signature), version 5.
    fn parameterized_iid(signature: &str) -> GUID {
        let mut data =
            vec![0x11, 0xF4, 0x7A, 0xD5, 0x7B, 0x73, 0x42, 0xC0, 0xAB, 0xAE, 0x87, 0x8B, 0x1E, 0x16, 0xAD, 0xEE];
        data.extend_from_slice(signature.as_bytes());
        let hash = sha1(&data);
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&hash[..16]);
        bytes[6] = (bytes[6] & 0x0F) | 0x50;
        bytes[8] = (bytes[8] & 0x3F) | 0x80;
        GUID::new(
            u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            u16::from_be_bytes([bytes[4], bytes[5]]),
            u16::from_be_bytes([bytes[6], bytes[7]]),
            [bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]],
        )
    }

    fn sha1(message: &[u8]) -> [u8; 20] {
        let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
        let mut data = message.to_vec();
        let bits = (message.len() as u64) * 8;
        data.push(0x80);
        while data.len() % 64 != 56 {
            data.push(0);
        }
        data.extend_from_slice(&bits.to_be_bytes());
        for chunk in data.chunks(64) {
            let mut w = [0u32; 80];
            for i in 0..16 {
                w[i] = u32::from_be_bytes([chunk[4 * i], chunk[4 * i + 1], chunk[4 * i + 2], chunk[4 * i + 3]]);
            }
            for i in 16..80 {
                w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
            }
            let [mut a, mut b, mut c, mut d, mut e] = h;
            for (i, word) in w.iter().enumerate() {
                let (f, k) = match i {
                    0..=19 => ((b & c) | (!b & d), 0x5A827999),
                    20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                    40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                    _ => (b ^ c ^ d, 0xCA62C1D6),
                };
                let temp = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(*word);
                e = d;
                d = c;
                c = b.rotate_left(30);
                b = a;
                a = temp;
            }
            for (slot, value) in h.iter_mut().zip([a, b, c, d, e]) {
                *slot = slot.wrapping_add(value);
            }
        }
        let mut out = [0u8; 20];
        for (i, word) in h.iter().enumerate() {
            out[4 * i..4 * i + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }
}
