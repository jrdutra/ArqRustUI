//! Example 18 — Multiple windows: main, owned, borderless,
//! translucent, full screen and background work without freezing the interface.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let main = Window::builder().title("Main window").size(440, 300).centered().build()?;
    let info = Label::builder().bounds(16, 12, 408, 40).parent(&main).build()?;
    let column = StackLayout::vertical().spacing(8).padding(Thickness::new(16, 60, 16, 16));
    let counter = Rc::new(Cell::new(0u32));

    let update = {
        let info = info.clone();
        move |open: i32| info.set_text(&format!("Secondary windows open: {open}\nMonitors: {}", Monitor::all().len()))
    };
    let open_count = Rc::new(Cell::new(0i32));
    update(0);

    let add = |caption: &str, action: Box<dyn Fn()>| -> GuiResult<()> {
        let button = Button::builder().text(caption).parent(&main).build()?;
        button.on_click(action);
        column.add_with(&button, LayoutParams::new().height(30));
        Ok(())
    };

    // Independent window: shows up on the taskbar and lives on its own.
    let (count, opened, refresh) = (counter.clone(), open_count.clone(), update.clone());
    add(
        "New independent window",
        Box::new(move || {
            count.set(count.get() + 1);
            let Ok(window) = Window::builder().title(&format!("Document {}", count.get())).size(300, 180).build()
            else {
                return;
            };
            let _ =
                Label::builder().text("Independent top-level window.").bounds(12, 12, 280, 20).parent(&window).build();
            opened.set(opened.get() + 1);
            refresh(opened.get());
            let (opened, refresh) = (opened.clone(), refresh.clone());
            window.on_destroy(move || {
                opened.set(opened.get() - 1);
                refresh(opened.get());
            });
            window.show();
        }),
    )?;

    // Owned window: always stays above the main one and closes with it.
    let owner = main.clone();
    add(
        "Tool palette (owned)",
        Box::new(move || {
            let Ok(palette) =
                Window::builder().title("Tools").size(180, 140).tool_window().resizable(false).owner(&owner).build()
            else {
                return;
            };
            for (index, tool) in ["Brush", "Eraser", "Text"].iter().enumerate() {
                let _ = ToggleButton::builder()
                    .text(tool)
                    .bounds(10, 10 + index as i32 * 36, 160, 30)
                    .parent(&palette)
                    .build();
            }
            palette.show();
        }),
    )?;

    // Borderless, translucent, always-on-top window that disappears on its own.
    add(
        "Floating notice (borderless)",
        Box::new(|| {
            let Ok(toast) = Window::builder()
                .size(260, 60)
                .borderless()
                .topmost(true)
                .opacity(0.9)
                .tool_window()
                .background(Color::rgb(30, 41, 59))
                .centered()
                .build()
            else {
                return;
            };
            if let Ok(label) = Label::builder()
                .text("Saved successfully ✓")
                .bounds(0, 20, 260, 24)
                .align(TextAlign::Center)
                .parent(&toast)
                .build()
            {
                label.set_foreground(Color::WHITE);
            }
            // DWM shadow on a borderless window, when available.
            let _ = toast.extend_frame_into_client(Thickness::all(1));
            toast.show();
            let _ = Timer::once(Duration::from_secs(2), move || toast.destroy());
        }),
    )?;

    let target = main.clone();
    add("Toggle full screen (F11)", Box::new(move || target.set_fullscreen(!target.is_fullscreen())))?;
    let target = main.clone();
    main.add_shortcut(Shortcut::key(Key::F11), move || target.set_fullscreen(!target.is_fullscreen()));

    // Slow work on another thread; the interface keeps responding.
    let progress = ProgressBar::builder().parent(&main).build()?;
    let (bar, text) = (progress.clone(), info.clone());
    let dispatcher = app.dispatcher();
    add(
        "Process in the background",
        Box::new(move || {
            let (bar, text, dispatcher) = (bar.clone(), text.clone(), dispatcher.clone());
            let started = spawn_local(async move {
                bar.set_value(0);
                // The control cannot go to the other thread: only the dispatcher does.
                let total = run_blocking(move || {
                    let mut sum = 0u64;
                    for step in 1..=20u64 {
                        std::thread::sleep(Duration::from_millis(60));
                        sum += step;
                        let _ = dispatcher.post(move || { /* progress messages could be sent here */ });
                    }
                    sum
                });
                // While the thread works, the bar is animated on the UI thread.
                for step in 1..=20 {
                    sleep(Duration::from_millis(60)).await;
                    bar.set_value(step * 5);
                }
                text.set_text(&format!("Processing finished: result {}", total.await));
            });
            if started.is_err() {
                MessageBox::error("Error", "Could not start the task.");
            }
        }),
    )?;
    column.add_with(&progress, LayoutParams::new().height(16));

    main.set_layout(column);
    main.show();
    app.run()
}
