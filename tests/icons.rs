//! Pacote de ícones embutido e exibição de ícones nos controles.

#![cfg(feature = "icons")]

mod common;

use arqrustui::ffi::user32::*;
use arqrustui::prelude::*;
use common::*;
use std::time::Duration;

/// Janela visível, sem bordas e sempre por cima, para conferir pixels desenhados.
fn visible_window(width: i32, height: i32) -> Window {
    let window = Window::builder()
        .size(width, height)
        .position(20, 20)
        .borderless()
        .topmost(true)
        .background(Color::WHITE)
        .build()
        .expect("janela");
    window.show();
    window
}

/// Cor de um pixel da área cliente (coordenadas lógicas).
fn pixel_at(widget: &impl Widget, x: i32, y: i32) -> Color {
    let scale = |v: i32| v * widget.dpi() as i32 / 96;
    // SAFETY: DC da própria janela, liberado logo em seguida.
    unsafe {
        let hdc = GetDC(widget.handle() as _);
        let value = arqrustui::ffi::gdi32::GetPixel(hdc, scale(x), scale(y));
        ReleaseDC(widget.handle() as _, hdc);
        Color::from_colorref(value)
    }
}

/// `true` se há algo diferente de branco no retângulo lógico (amostragem em grade).
fn has_ink(widget: &impl Widget, area: Rect) -> bool {
    (area.y..area.bottom()).step_by(2).any(|y| {
        (area.x..area.right()).step_by(2).any(|x| {
            let c = pixel_at(widget, x, y);
            c.r < 235 || c.g < 235 || c.b < 235
        })
    })
}

#[test]
fn every_builtin_icon_loads_from_ico_and_png() {
    with_app(|_| {
        for asset in icons::all() {
            for size in [16, 20, 32, 48] {
                let icon = asset.icon(size).unwrap_or_else(|e| panic!("{}@{size}: {e}", asset.name()));
                assert_eq!(icon.size(), Size::new(size, size), "{}", asset.name());
            }
            let image = asset.image().unwrap_or_else(|e| panic!("{} png: {e}", asset.name()));
            assert_eq!(image.size(), asset.png_size(), "{}", asset.name());
        }
    });
}

#[test]
fn social_pack_loads_all_sizes_and_keeps_existing_logo_variants() {
    with_app(|_| {
        let social: Vec<_> = icons::in_category(IconCategory::Social).collect();
        assert_eq!(social.len(), 58);
        for asset in social {
            for size in [16, 24, 32, 48, 64, 128, 256] {
                let icon = asset.icon(size).unwrap_or_else(|e| panic!("{}@{size}: {e}", asset.name()));
                assert_eq!(icon.size(), Size::new(size, size));
            }
            assert_eq!(asset.image().unwrap().size(), asset.png_size());
        }
        for (original, variant, alias) in [
            (&icons::GITHUB, &icons::GITHUB_SOCIAL, "21_GitHub"),
            (&icons::GITLAB, &icons::GITLAB_SOCIAL, "22_GitLab"),
            (&icons::THREADS, &icons::THREADS_ALTERNATE, "31_Threads_Alternate"),
        ] {
            assert_ne!(original.ico_bytes(), variant.ico_bytes());
            assert_eq!(icons::find(alias), Some(variant));
            assert_eq!(icons::find(original.name()), Some(original));
        }
        assert_eq!(icons::find("09_Threads"), Some(&icons::THREADS));
        assert_eq!(icons::find("Twitter"), Some(&icons::X_TWITTER));
        assert_eq!(icons::find("YouTube Shorts"), Some(&icons::YOUTUBE_SHORTS));
    });
}

#[test]
fn bevel_social_icons_have_png_512_svg_and_searchable_new_platforms() {
    with_app(|_| {
        for name in [
            "baidu_tieba",
            "bereal",
            "bilibili",
            "bluesky",
            "discord",
            "douyin",
            "messenger",
            "facebook",
            "instagram",
            "kakaotalk",
            "kick",
            "kuaishou_kwai",
            "lemon8",
            "likee",
            "line",
            "linkedin",
            "mastodon",
            "nextdoor",
            "odnoklassniki",
            "pinterest",
            "qq",
            "quora",
            "reddit",
            "rumble",
            "signal",
            "snapchat",
            "telegram",
            "threads",
            "tiktok",
            "tumblr",
            "twitch",
            "viber",
            "vk_vkontakte",
            "wechat",
            "weibo",
            "whatsapp",
            "x_twitter",
            "xiaohongshu_rednote",
            "youtube",
            "zhihu",
        ] {
            let asset = icons::find(name).unwrap_or_else(|| panic!("ícone ausente: {name}"));
            assert_eq!(asset.category(), Some(IconCategory::Social));
            assert_eq!(asset.png_size(), Size::new(512, 512), "{name}");
            assert_eq!(asset.image().unwrap().size(), Size::new(512, 512), "{name}");
            let svg = asset.svg_image().unwrap().expect("SVG fornecido");
            assert_eq!(svg.size(), Size::new(256, 256), "{name}");
            assert_eq!(svg.render(32, 32).unwrap().size(), Size::new(32, 32), "{name}");
        }
        assert_eq!(icons::find("facebook_messenger"), Some(&icons::MESSENGER));
        assert_eq!(icons::find("26_messenger"), Some(&icons::MESSENGER));
        assert_eq!(icons::find("Kwai"), Some(&icons::KUAISHOU_KWAI));
        assert_eq!(icons::find("VK"), Some(&icons::VK_VKONTAKTE));
        assert_eq!(icons::find("Rednote"), Some(&icons::XIAOHONGSHU_REDNOTE));
        assert_eq!(icons::find("Be Real"), Some(&icons::BEREAL));
    });
}

#[test]
fn icons_are_cached_scaled_and_have_disabled_variants() {
    with_app(|_| {
        let first = icons::FOLDER.icon(24).unwrap();
        assert_eq!(first.raw(), icons::FOLDER.icon(24).unwrap().raw(), "cache por thread");
        assert_ne!(first.raw(), icons::FOLDER.icon(32).unwrap().raw());
        let window = test_window();
        let expected = 16 * window.dpi() as i32 / 96;
        assert_eq!(icons::SAVE.icon_for(&window, 16).unwrap().size(), Size::new(expected, expected));
        let disabled = first.disabled().unwrap();
        assert_eq!(disabled.size(), first.size());
        assert_ne!(disabled.raw(), first.raw());
        assert!(icons::SAVE.icon(0).is_err());
        assert!(Icon::from_ico_bytes(b"isto nao e um ico", 16).is_err());
        assert!(PlusImage::from_bytes(b"nem um png").is_err());
        assert!(PlusImage::from_bytes(&[]).is_err());
    });
}

#[test]
fn icons_reach_pixels_through_canvas_and_gdiplus() {
    with_app(|_| {
        let mut gdi = None;
        Bitmap::render(32, 32, |canvas| {
            canvas.clear(Color::WHITE);
            canvas.draw_icon(&icons::ADD, Point::new(0, 0), 32);
            gdi = canvas.pixel(Point::new(9, 9));
        })
        .unwrap();
        let disc = gdi.expect("pixel");
        assert!(disc.g > disc.r && disc.g > disc.b, "o disco de ADD é verde: {disc:?}");

        let png = icons::CLOSE.image().unwrap();
        let mut plus = None;
        Bitmap::render(64, 64, |canvas| {
            canvas.clear(Color::WHITE);
            canvas.with_gdiplus(|g| g.draw_image(&png, Rect::new(0, 0, 64, 64))).unwrap();
            plus = canvas.pixel(Point::new(32, 52));
        })
        .unwrap();
        let ring = plus.expect("pixel");
        assert!(ring.r > 150 && ring.g < 120, "o anel de CLOSE é vermelho: {ring:?}");

        // PNG → ícone por GDI+ (ícones próprios sem .ico).
        static PNG_ONLY: IconAsset = IconAsset::new(
            "so_png",
            &[],
            match arqrustui_icons::CHECK.1 {
                Some(png) => png,
                None => &[],
            },
            None,
        );
        assert_eq!(PNG_ONLY.icon(24).unwrap().size(), Size::new(24, 24));
        assert_eq!(png.to_icon(40).unwrap().size(), Size::new(40, 40));
    });
}

#[test]
fn assets_can_be_saved_in_every_format() {
    let dir = std::env::temp_dir().join("arqrustui-icones");
    std::fs::create_dir_all(&dir).unwrap();
    for extension in ["ico", "png", "svg"] {
        let path = dir.join(format!("save.{extension}"));
        icons::SAVE.save(&path).unwrap();
        let expected = match extension {
            "ico" => icons::SAVE.ico_bytes(),
            "png" => icons::SAVE.png_bytes(),
            _ => icons::SAVE.svg_bytes().unwrap(),
        };
        assert_eq!(std::fs::read(&path).unwrap(), expected);
    }
    assert!(matches!(icons::DOCKER.save(dir.join("docker.svg")), Err(GuiError::Unsupported(_))));
    assert!(icons::DOCKER.save(dir.join("docker.bmp")).is_err());
}

#[test]
fn button_icon_native_and_custom_drawn() {
    with_app(|app| {
        let window = visible_window(260, 60);
        let button = Button::builder()
            .text("Salvar")
            .icon(&icons::SAVE)
            .bounds(10, 10, 110, 32)
            .parent(&window)
            .build()
            .unwrap();
        assert!(button.has_icon());
        if button.is_icon_native() {
            // Common Controls v6: o botão continua nativo e ícone e texto são desenhados
            // sobre o fundo do tema, no aviso NM_CUSTOMDRAW.
            assert_eq!(native_style(&button) & BS_TYPEMASK, BS_PUSHBUTTON);
        } else {
            // Sem a v6 (por exemplo, no Wine) o botão passa a ser desenhado pela biblioteca.
            assert_eq!(native_style(&button) & BS_TYPEMASK, BS_OWNERDRAW);
        }
        // Padrão: 60% da altura (32 → 19), grupo ícone + texto centralizado e afastado da
        // borda — o ícone não fica mais encostado no canto esquerdo.
        assert_eq!(button.icon_size(), Some(19));
        pump_until(app, Duration::from_millis(300), || false);
        assert!(!has_ink(&window, Rect::new(13, 16, 12, 20)), "nada encostado na borda esquerda");
        assert!(has_ink(&window, Rect::new(30, 16, 22, 20)), "ícone desenhado no grupo centralizado");
        button.set_icon_size(24);
        assert_eq!(button.icon_size(), Some(24));
        button.set_icon_size(None);
        assert_eq!(button.icon_size(), Some(19));
        button.set_size(110, 40);
        assert_eq!(button.icon_size(), Some(24), "o automático acompanha a altura");
        button.set_icon_position(IconPosition::Right);
        button.set_icon(&icons::PRINTER).unwrap();
        button.clear_icon();
        assert!(!button.has_icon());
        assert_eq!(native_style(&button) & BS_TYPEMASK, BS_PUSHBUTTON, "volta a ser um botão nativo comum");

        // Botão desenhado pela biblioteca: o ícone é pintado junto do texto.
        let styled = Button::builder()
            .text("OK")
            .icon(&icons::CHECK)
            .icon_size(24)
            .style(ButtonStyle::from_accent(Color::rgb(30, 90, 200), Color::WHITE))
            .bounds(140, 10, 110, 40)
            .parent(&window)
            .build()
            .unwrap();
        pump_until(app, Duration::from_millis(300), || false);
        let green = (150..180).flat_map(|x| (14..46).map(move |y| (x, y))).any(|(x, y)| {
            let c = pixel_at(&window, x, y);
            c.g > 150 && c.r < 120 && c.b < 120
        });
        assert!(green, "o ícone verde de CHECK aparece no botão desenhado");
        styled.set_enabled(false);
        pump_until(app, Duration::from_millis(200), || false);
    });
}

#[test]
fn label_with_icon_is_owner_drawn_and_reverts() {
    with_app(|app| {
        let window = visible_window(240, 80);
        let label = Label::builder()
            .text("&Aviso")
            .align(TextAlign::Center)
            .icon(&icons::WARNING)
            .icon_size(32)
            .bounds(0, 0, 240, 40)
            .parent(&window)
            .build()
            .unwrap();
        assert_eq!(native_style(&label) & SS_TYPEMASK, SS_OWNERDRAW);
        pump_until(app, Duration::from_millis(300), || false);
        assert!(has_ink(&window, Rect::new(60, 4, 120, 32)), "ícone e texto centralizados");
        let plain = Label::builder().text("Sem ícone").bounds(0, 44, 240, 20).parent(&window).build().unwrap();
        plain.set_icon(&icons::INFO).unwrap();
        assert!(plain.has_icon());
        label.clear_icon();
        assert!(!label.has_icon());
        assert_eq!(native_style(&label) & SS_TYPEMASK, SS_CENTER, "estilo original restaurado");
        pump_until(app, Duration::from_millis(200), || false);
    });
}

fn native_style(widget: &impl Widget) -> u32 {
    // SAFETY: consulta de estilo de uma janela viva da thread de teste.
    unsafe { GetWindowLongPtrW(widget.handle() as _, GWL_STYLE) as u32 }
}

#[test]
fn picture_box_scales_icons_and_images() {
    with_app(|app| {
        let window = visible_window(420, 320);
        let small = PictureBox::builder().icon(&icons::GLOBE).bounds(0, 0, 48, 48).parent(&window).build().unwrap();
        let px = 48 * window.dpi() as i32 / 96;
        assert_eq!(small.current_icon().unwrap().size(), Size::new(px, px));
        // Maior que 256 px: desenhado a partir do PNG pelo GDI+.
        let large = PictureBox::builder().icon(&icons::GLOBE).bounds(60, 0, 300, 300).parent(&window).build().unwrap();
        pump_until(app, Duration::from_millis(300), || false);
        assert!(has_ink(&window, Rect::new(160, 100, 100, 100)), "PNG grande desenhado");
        large.set_image(icons::PICTURE.image().unwrap());
        assert!(large.has_content());
        large.clear();
        assert!(!large.has_content());
        small.set_icon_size(Some(16));
        assert_eq!(small.current_icon().unwrap().size(), Size::new(16 * px / 48, 16 * px / 48));
    });
}

#[test]
fn tree_list_and_tabs_share_icons_through_managed_image_lists() {
    with_app(|_| {
        let window = test_window();
        let tree = TreeView::builder().bounds(0, 0, 200, 200).parent(&window).build().unwrap();
        let root = tree.add_root("Projeto").unwrap();
        let child = tree.add_child(root, "main.rs").unwrap();
        let plain = tree.add_child(root, "sem ícone").unwrap();
        tree.set_item_icons(root, &icons::FOLDER, &icons::FOLDER_OPEN).unwrap();
        tree.set_item_icon(child, &icons::CODE_FILE).unwrap();
        tree.set_item_icon(plain, &icons::FOLDER).unwrap();
        assert_eq!(tree.item_image(root), Some(0));
        assert_eq!(tree.item_image(child), Some(2));
        assert_eq!(tree.item_image(plain), Some(0), "o mesmo ícone reaproveita a posição");
        tree.clear_item_icon(plain);
        assert_eq!(tree.item_image(plain), None);
        tree.set_icon_size(24).unwrap();
        assert_eq!(tree.item_image(child), Some(2), "índices preservados ao mudar o tamanho");

        let list = ListView::builder().bounds(0, 0, 200, 200).parent(&window).build().unwrap();
        list.add_column("Nome", 120, ColumnAlign::Left);
        list.add_row(&["a.json"]);
        list.add_row(&["b.yaml"]);
        assert_eq!(list.row_image(0), None, "linhas nascem sem imagem");
        list.set_row_icon(0, &icons::JSON_FILE).unwrap();
        list.set_row_icon(1, &Icon::system(SystemIcon::Information).unwrap()).unwrap();
        assert_eq!((list.row_image(0), list.row_image(1)), (Some(0), Some(1)));
        list.clear_row_icon(0);
        assert_eq!(list.row_image(0), None);

        let tabs = TabControl::builder().bounds(0, 0, 300, 200).parent(&window).build().unwrap();
        tabs.add_tab("Geral").unwrap();
        tabs.add_tab("Rede").unwrap();
        tabs.set_tab_icon(1, &icons::NETWORK).unwrap();
        assert_eq!((tabs.tab_image(0), tabs.tab_image(1)), (None, Some(0)));
        assert!(tabs.set_tab_icon(7, &icons::NETWORK).is_err());
        tabs.clear_tab_icon(1);
        assert_eq!(tabs.tab_image(1), None);
    });
}

#[test]
fn data_grid_icons_follow_sorting_and_virtual_providers() {
    with_app(|app| {
        let window = visible_window(320, 200);
        let grid = DataGrid::builder().bounds(0, 0, 320, 200).parent(&window).build().unwrap();
        grid.add_column("Arquivo", 200, ColumnAlign::Left);
        grid.add_row(&["c"]);
        grid.add_row(&["a"]);
        grid.add_row(&["b"]);
        grid.set_row_icon(0, &icons::XML_FILE).unwrap();
        grid.set_row_icon(1, &icons::JSON_FILE).unwrap();
        assert!(grid.set_row_icon(9, &icons::XML_FILE).is_err());
        grid.sort_by(0, true);
        let names: Vec<String> = (0..3).map(|r| grid.cell(r, 0).unwrap()).collect();
        assert_eq!(names, ["a", "b", "c"]);
        assert_eq!((grid.row_image(0), grid.row_image(1), grid.row_image(2)), (Some(1), None, Some(0)));
        grid.remove_row(0);
        assert_eq!(grid.row_image(1), Some(0));
        grid.clear_row_icon(1);
        assert_eq!(grid.row_image(1), None);

        grid.set_virtual_source(1000, |row, _| format!("linha {row}"));
        grid.set_icon_provider(Some(|row: usize| (row % 2 == 0).then_some(&icons::FOLDER))).unwrap();
        let folder = grid.row_image(10);
        assert!(folder.is_some());
        assert_eq!(grid.row_image(12), folder, "mesma origem, mesma posição na lista");
        assert_eq!(grid.row_image(11), None);
        pump_until(app, Duration::from_millis(200), || false);
        grid.set_icon_provider(None::<fn(usize) -> Option<&'static IconAsset>>).unwrap();
        assert_eq!(grid.row_image(10), None);
    });
}

#[test]
fn combo_and_list_boxes_draw_icon_items() {
    with_app(|app| {
        let window = visible_window(260, 200);
        let combo = ComboBox::builder().with_icons(true).bounds(0, 0, 200, 24).parent(&window).build().unwrap();
        let first = combo.add_item_with_icon("Python", &icons::PYTHON).unwrap();
        combo.add_item("Sem ícone");
        combo.add_item_with_icon("Java", &icons::JAVA).unwrap();
        assert_eq!(combo.item_image(first), Some(0));
        assert_eq!(combo.item_image(1), None);
        assert_eq!(combo.item_image(2), Some(1));
        let item_height = send(&combo, CB_GETITEMHEIGHT, 0, 0) as i32;
        assert!(item_height >= 16 * window.dpi() as i32 / 96, "altura comporta o ícone: {item_height}");
        combo.set_selected_index(Some(0));
        combo.clear_item_icon(2).unwrap();
        assert_eq!(combo.item_image(2), None);
        assert!(combo.set_item_icon(50, &icons::PYTHON).is_err());

        let list = ListBox::builder().with_icons(true).bounds(0, 40, 200, 120).parent(&window).build().unwrap();
        list.add_item_with_icon("Pasta", &icons::FOLDER).unwrap();
        list.add_item_with_icon("Arquivo", &icons::FILE).unwrap();
        list.set_item_icon(0, &icons::FOLDER_OPEN).unwrap();
        assert_eq!(list.item_image(0), Some(2));
        pump_until(app, Duration::from_millis(300), || false);
        assert!(has_ink(&window, Rect::new(2, 42, 20, 20)), "ícone do primeiro item desenhado");

        // Caixas não editáveis já aceitam ícones; nas editáveis, sem `with_icons(true)`,
        // o recurso é recusado de forma explícita.
        let plain = ComboBox::builder().parent(&window).build().unwrap();
        assert_eq!(plain.add_item_with_icon("x", &icons::FILE).unwrap(), 0);
        let editable = ComboBox::builder().editable(true).parent(&window).build().unwrap();
        assert!(matches!(editable.add_item_with_icon("x", &icons::FILE), Err(GuiError::Unsupported(_))));
        assert_eq!(editable.count(), 0);
    });
}

#[test]
fn bars_menus_windows_tray_and_dialogs_accept_icon_assets() {
    with_app(|app| {
        let window = test_window();
        window.set_icon(&icons::COMPUTER);
        let small = send(&window, WM_GETICON, ICON_SMALL, 0);
        let big = send(&window, WM_GETICON, ICON_BIG, 0);
        assert!(small != 0 && big != 0 && small != big, "ícones pequeno e grande distintos");

        let built = Window::builder().title("Com ícone").icon(&icons::CLOUD).build().unwrap();
        assert_ne!(send(&built, WM_GETICON, ICON_BIG, 0), 0);

        let toolbar = ToolBar::new(&window).unwrap();
        toolbar.add_button("Abrir", Some(&icons::FOLDER_OPEN), || {}).unwrap();
        toolbar.add_button("Info", Some(&Icon::system(SystemIcon::Information).unwrap()), || {}).unwrap();
        toolbar.add_toggle_button("Fixar", Some(&icons::PIN), |_| {}).unwrap();

        let status = StatusBar::new(&window).unwrap();
        status.set_parts(&[120, -1]);
        status.set_part_icon(1, &icons::WIFI).unwrap();
        assert!(status.has_part_icon(1) && !status.has_part_icon(0));
        status.clear_part_icon(1);
        assert!(!status.has_part_icon(1));

        let menu = Menu::new().unwrap();
        let item = menu.add_item("Salvar", || {}).unwrap();
        item.set_icon(&icons::SAVE).unwrap();

        match TrayIcon::new(&icons::CLOUD_SYNC, "ArqRustUI ícones") {
            Ok(tray) => tray.set_icon(&icons::CLOUD_LOCK).unwrap(),
            Err(GuiError::Unsupported(_)) => {}
            Err(other) => panic!("{other}"),
        }
        pump_until(app, Duration::from_millis(100), || false);
    });
}

#[test]
fn find_and_categories_are_usable_from_applications() {
    assert_eq!(icons::find("json").map(|i| i.name()), Some("json_file"));
    assert_eq!(icons::find("01_10_pasta"), Some(&icons::FOLDER));
    assert!(icons::in_category(IconCategory::Technologies).any(|i| i == &icons::DOCKER));
    for category in IconCategory::ALL {
        assert!(!category.label().is_empty());
        assert!(icons::in_category(category).next().is_some(), "{category:?} vazia");
    }
}
