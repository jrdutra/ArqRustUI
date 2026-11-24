//! Embedded icon catalog. Bytes from `arqrustui-icons` (collecting `crates/arqrustui-icons-data1` through
//! `arqrustui-icons-data8`); names, categories and aliases live here.
//!
//! To add an icon, see "Adding an icon" in `assets/icons/README.md`: data crate stores files/byte constant; add an
//! `icon!` line and [`ALL`] entry here in alphabetical order.

use super::{IconAsset, IconCategory};

macro_rules! icon {
    ($constant:ident, $name:literal, $category:ident, [$($alias:literal),*]) => {
        #[doc = concat!(
            "Icon `", $name, "` — category [`", stringify!($category), "`](IconCategory::", stringify!($category), ")."
            $(, " Aliases: `", $alias, "`")*
        )]
        pub static $constant: IconAsset = IconAsset::builtin(
            $name,
            IconCategory::$category,
            &[$($alias),*],
            arqrustui_icons::$constant,
        );
    };
}

icon!(SEVEN_ZIP_FILE, "7z_file", FileTypes, ["7z", "seven_zip_file"]);
icon!(AAC_FILE, "aac_file", FileTypes, ["aac"]);
icon!(ACCESSIBILITY, "accessibility", People, []);
icon!(ADD, "add", Actions, []);
icon!(AI_FILE, "ai_file", FileTypes, ["ai", "illustrator_file"]);
icon!(ALARM_CLOCK, "alarm_clock", Office, ["alarm", "22_despertador"]);
icon!(ANGULAR, "angular", Technologies, ["04_11_angular"]);
icon!(API_HUB, "api_hub", Development, []);
icon!(APK_FILE, "apk_file", FileTypes, ["apk"]);
icon!(ARCHIVE, "archive", Files, []);
icon!(AVI_FILE, "avi_file", FileTypes, ["avi"]);
icon!(AZURE_DEVOPS, "azure_devops", Technologies, ["03_02_azure_devops"]);
icon!(BACK, "back", Actions, ["arrow_left"]);
icon!(BAIDU_TIEBA, "baidu_tieba", Social, ["40_baidu_tieba"]);
icon!(BAR_CHART_REPORT, "bar_chart_report", Data, ["report", "29_relatorio_barras"]);
icon!(BARCODE, "barcode", Commerce, ["57_codigo_de_barras"]);
icon!(BARCODE_SCANNER, "barcode_scanner", Commerce, []);
icon!(BAT_FILE, "bat_file", FileTypes, ["bat", "batch_file"]);
icon!(BATTERY, "battery", Hardware, []);
icon!(BATTERY_CHARGING, "battery_charging", Hardware, ["charging_battery"]);
icon!(BEHANCE, "behance", Social, ["19_behance"]);
icon!(BEREAL, "bereal", Social, ["be_real"]);
icon!(BILIBILI, "bilibili", Social, []);
icon!(BLOCKED, "blocked", Status, ["forbidden", "denied", "77_bloqueado"]);
icon!(BLUESKY, "bluesky", Social, ["39_bluesky"]);
icon!(BMP_FILE, "bmp_file", FileTypes, ["bmp", "bitmap_file"]);
icon!(BRIEFCASE, "briefcase", Office, ["work"]);
icon!(BUG, "bug", Development, ["debug", "07_09_debug"]);
icon!(BURN_DISC, "burn_disc", Media, ["42_gravar_disco"]);
icon!(C_FILE, "c_file", FileTypes, ["c"]);
icon!(CALCULATOR, "calculator", Office, []);
icon!(CALENDAR, "calendar", Office, ["21_calendario"]);
icon!(CAMERA, "camera", Media, ["photo_camera", "33_camera_fotografica"]);
icon!(CHAT, "chat", Communication, ["messages", "chat_bubble"]);
icon!(CHECK, "check", Status, ["ok", "success"]);
icon!(CHECKLIST_DONE, "checklist_done", Office, ["checklist", "27_checklist_aprovado", "07_10_checklist"]);
icon!(CHECKLIST_PENDING, "checklist_pending", Office, ["28_checklist_pendente"]);
icon!(CLAPPERBOARD, "clapperboard", Media, ["movie", "35_video_claquete"]);
icon!(CLIPBOARD, "clipboard", Office, []);
icon!(CLOCK, "clock", Office, ["time"]);
icon!(CLOSE, "close", Actions, ["cancel", "error"]);
icon!(CLOUD, "cloud", Network, ["06_nuvem", "02_07_nuvem"]);
icon!(CLOUD_DOWNLOAD, "cloud_download", Network, ["07_download_nuvem"]);
icon!(CLOUD_LOCK, "cloud_lock", Network, []);
icon!(CLOUD_SETTINGS, "cloud_settings", Network, []);
icon!(CLOUD_SYNC, "cloud_sync", Network, []);
icon!(CLOUD_TRANSFER, "cloud_transfer", Network, ["02_08_sincronizacao_nuvem"]);
icon!(CLOUD_UPLOAD, "cloud_upload", Network, ["08_upload_nuvem"]);
icon!(CODE_EDITOR, "code_editor", Development, ["02_01_editor_codigo"]);
icon!(CODE_FILE, "code_file", Files, ["source_file", "01_04_arquivo_codigo"]);
icon!(CODE_WINDOW, "code_window", Development, ["ide", "01_06_ide_codigo"]);
icon!(COMPASS, "compass", Navigation, []);
icon!(
    COMPRESSED_FILES,
    "compressed_files",
    Files,
    ["compressed_file", "48_arquivos_compactados", "08_05_arquivo_compactado"]
);
icon!(COMPUTER, "computer", Devices, ["desktop", "pc", "01_computador_desktop", "01_01_desktop_servidor"]);
icon!(CONTACTS, "contacts", Communication, ["address_book", "19_contatos"]);
icon!(COPY, "copy", Actions, []);
icon!(CPP, "cpp", Technologies, ["04_03_cpp"]);
icon!(CPP_FILE, "cpp_file", FileTypes, ["cplusplus_file"]);
icon!(CPU, "cpu", Hardware, ["processor", "chip", "03_09_processador"]);
icon!(CREDIT_CARD, "credit_card", Commerce, ["payment"]);
icon!(CS_FILE, "cs_file", FileTypes, ["cs", "csharp_file"]);
icon!(CSHARP, "csharp", Technologies, ["04_04_csharp"]);
icon!(CSS3, "css3", Technologies, ["04_10_css3"]);
icon!(CSS_FILE, "css_file", FileTypes, ["css"]);
icon!(CSV_FILE, "csv_file", FileTypes, ["csv"]);
icon!(CUT, "cut", Actions, []);
icon!(DATABASE, "database", Data, ["05_banco_de_dados", "02_04_banco_dados"]);
icon!(DATABASE_SEARCH, "database_search", Data, []);
icon!(DATABASE_SYNC, "database_sync", Data, []);
icon!(DB_FILE, "db_file", FileTypes, ["db", "database_file"]);
icon!(DELETE, "delete", Actions, ["trash"]);
icon!(DELIVERY_TRUCK, "delivery_truck", Commerce, ["shipping"]);
icon!(DEVELOPER, "developer", People, ["01_03_desenvolvedor"]);
icon!(DEVOPS, "devops", Development, ["03_07_devops_infinito"]);
icon!(DISC_DOWNLOAD, "disc_download", Media, ["45_baixar_disco"]);
icon!(DISCORD, "discord", Social, ["11_discord"]);
icon!(DLL_FILE, "dll_file", FileTypes, ["dll"]);
icon!(DOC_FILE, "doc_file", FileTypes, ["doc"]);
icon!(DOCKER, "docker", Technologies, ["03_04_docker"]);
icon!(DOCX_FILE, "docx_file", FileTypes, ["docx", "word_file"]);
icon!(DOTNET, "dotnet", Technologies, ["04_05_dotnet"]);
icon!(DOUYIN, "douyin", Social, []);
icon!(DOWN, "down", Actions, ["arrow_down"]);
icon!(DOWNLOAD, "download", Actions, []);
icon!(DOWNLOAD_WINDOW, "download_window", Development, []);
icon!(DRIBBBLE, "dribbble", Social, ["20_dribbble"]);
icon!(DUAL_MONITORS, "dual_monitors", Devices, ["03_monitores_duplos"]);
icon!(EDIT, "edit", Actions, ["pencil"]);
icon!(EDIT_NOTE, "edit_note", Office, ["20_editar_anotacao"]);
icon!(ELASTICSEARCH, "elasticsearch", Technologies, ["05_10_elasticsearch"]);
icon!(EMAIL, "email", Communication, ["16_email_arroba"]);
icon!(EMAIL_RECEIVE, "email_receive", Communication, ["18_receber_email"]);
icon!(EMAIL_SEND, "email_send", Communication, ["17_enviar_email"]);
icon!(EMOJI_GRIN, "emoji_grin", Emoji, ["sorriso_dentes", "01_sorriso_dentes"]);
icon!(EMOJI_HAND_OVER_MOUTH, "emoji_hand_over_mouth", Emoji, ["mao_na_boca", "29_mao_na_boca"]);
icon!(EMOJI_HUG, "emoji_hug", Emoji, ["abraco", "26_abraco"]);
icon!(EMOJI_JOY, "emoji_joy", Emoji, ["rindo_lagrimas", "tears_of_joy", "03_rindo_lagrimas"]);
icon!(EMOJI_KISS, "emoji_kiss", Emoji, ["mandando_beijo", "beijo", "14_mandando_beijo"]);
icon!(EMOJI_MIND_BLOWN, "emoji_mind_blown", Emoji, ["cabeca_explodindo", "17_cabeca_explodindo"]);
icon!(EMOJI_NAUSEATED, "emoji_nauseated", Emoji, ["enjoado", "28_enjoado"]);
icon!(EMOJI_NERD, "emoji_nerd", Emoji, ["nerd", "15_nerd"]);
icon!(EMOJI_NERVOUS, "emoji_nervous", Emoji, ["nervoso", "21_nervoso"]);
icon!(EMOJI_PARTY, "emoji_party", Emoji, ["festa", "16_festa"]);
icon!(EMOJI_POOP, "emoji_poop", Emoji, ["coco_sorridente", "30_coco_sorridente"]);
icon!(EMOJI_RELIEVED, "emoji_relieved", Emoji, ["aliviado", "20_aliviado"]);
icon!(EMOJI_SCREAM, "emoji_scream", Emoji, ["gritando", "25_gritando"]);
icon!(EMOJI_SMILE, "emoji_smile", Emoji, ["sorriso_aberto", "smile", "02_sorriso_aberto"]);
icon!(EMOJI_STAR_STRUCK, "emoji_star_struck", Emoji, ["olhos_estrelas", "07_olhos_estrelas"]);
icon!(EMOJI_SURPRISED, "emoji_surprised", Emoji, ["surpreso", "09_surpreso"]);
icon!(EMOJI_SUSPICIOUS, "emoji_suspicious", Emoji, ["desconfiado", "19_desconfiado"]);
icon!(EMOJI_THINKING, "emoji_thinking", Emoji, ["pensativo", "thinking", "08_pensativo"]);
icon!(EMOJI_TONGUE, "emoji_tongue", Emoji, ["mostrando_lingua", "13_mostrando_lingua"]);
icon!(EMOJI_WINK, "emoji_wink", Emoji, ["piscadinha", "wink", "04_piscadinha"]);
icon!(EPS_FILE, "eps_file", FileTypes, ["eps"]);
icon!(EXE_FILE, "exe_file", FileTypes, ["exe", "executable_file"]);
icon!(EYEDROPPER, "eyedropper", Design, ["color_picker"]);
icon!(FACEBOOK, "facebook", Social, ["03_facebook"]);
icon!(FAVORITE, "favorite", Status, ["star", "69_favorito"]);
icon!(FAVORITE_ADD, "favorite_add", Status, ["70_adicionar_favorito"]);
icon!(FAVORITE_REMOVE, "favorite_remove", Status, ["71_remover_favorito"]);
icon!(FILE, "file", Files, ["document"]);
icon!(FILM_REEL, "film_reel", Media, ["film", "36_rolo_de_filme"]);
icon!(FIREWALL, "firewall", Security, ["63_firewall_ativo"]);
icon!(FLAC_FILE, "flac_file", FileTypes, ["flac"]);
icon!(FLAG_BLUE, "flag_blue", Status, ["flag", "72_bandeira_azul"]);
icon!(FLAG_CHECK, "flag_check", Status, ["75_bandeira_confirmada"]);
icon!(FLAG_ERROR, "flag_error", Status, ["76_bandeira_rejeitada"]);
icon!(FLAG_GREEN, "flag_green", Status, ["74_bandeira_verde"]);
icon!(FLAG_RED, "flag_red", Status, ["73_bandeira_vermelha"]);
icon!(FLICKR, "flickr", Social, ["18_flickr"]);
icon!(FLOWCHART, "flowchart", Office, ["workflow", "02_03_fluxograma"]);
icon!(FOLDER, "folder", Files, ["directory", "01_10_pasta"]);
icon!(FOLDER_OPEN, "folder_open", Files, ["folders", "01_11_pastas"]);
icon!(FORWARD, "forward", Actions, ["arrow_right"]);
icon!(FOUNTAIN_PEN, "fountain_pen", Design, ["pen", "signature"]);
icon!(GAMEPAD, "gamepad", Entertainment, ["controller"]);
icon!(GIF_FILE, "gif_file", FileTypes, ["gif"]);
icon!(GIFT, "gift", Commerce, ["present", "gift_box"]);
icon!(GIT, "git", Development, ["02_09_git"]);
icon!(GITHUB, "github", Technologies, ["02_10_github"]);
icon!(GITHUB_SOCIAL, "github_social", Social, ["21_github", "social_github"]);
icon!(GITLAB, "gitlab", Technologies, ["03_01_gitlab"]);
icon!(GITLAB_SOCIAL, "gitlab_social", Social, ["22_gitlab", "social_gitlab"]);
icon!(GLOBAL_NETWORK, "global_network", Network, ["09_rede_global"]);
icon!(GLOBE, "globe", Network, ["internet", "world", "07_04_internet"]);
icon!(GO_FILE, "go_file", FileTypes, ["go", "golang_file"]);
icon!(GOOGLE_MEET, "google_meet", Social, ["29_google_meet"]);
icon!(GPS_NAVIGATION, "gps_navigation", Navigation, ["gps", "navigation_screen"]);
icon!(GZ_FILE, "gz_file", FileTypes, ["gz", "gzip_file"]);
icon!(H_FILE, "h_file", FileTypes, ["h", "header_file"]);
icon!(HAMMER, "hammer", Office, []);
icon!(HAND_BACKHAND_POINT, "hand_backhand_point", Gestures, ["apontando_lado", "18_apontando_lado"]);
icon!(HAND_CALL_ME, "hand_call_me", Gestures, ["me_liga", "11_me_liga"]);
icon!(HAND_CLAP, "hand_clap", Gestures, ["aplausos", "clap", "12_aplausos"]);
icon!(HAND_FINGERS_CROSSED, "hand_fingers_crossed", Gestures, ["dedos_cruzados", "15_dedos_cruzados"]);
icon!(HAND_FIST, "hand_fist", Gestures, ["punho_fechado", "16_punho_fechado"]);
icon!(HAND_FLEXED_BICEPS, "hand_flexed_biceps", Gestures, ["braco_forte", "muscle", "28_braco_forte"]);
icon!(HAND_LOVE_YOU, "hand_love_you", Gestures, ["eu_te_amo", "23_eu_te_amo"]);
icon!(HAND_OK, "hand_ok", Gestures, ["ok_hand", "03_ok"]);
icon!(HAND_OPEN_HANDS, "hand_open_hands", Gestures, ["maos_abertas", "25_maos_abertas"]);
icon!(HAND_PALMS_UP, "hand_palms_up", Gestures, ["maos_em_concha", "27_maos_em_concha"]);
icon!(HAND_PEACE, "hand_peace", Gestures, ["paz", "victory", "04_paz"]);
icon!(HAND_PINCH, "hand_pinch", Gestures, ["beliscando", "20_beliscando"]);
icon!(HAND_PINCHED_FINGERS, "hand_pinched_fingers", Gestures, ["dedos_juntos", "19_dedos_juntos"]);
icon!(HAND_POINT_LEFT, "hand_point_left", Gestures, ["10_apontando_esquerda"]);
icon!(HAND_POINT_UP, "hand_point_up", Gestures, ["apontando_cima", "07_apontando_cima"]);
icon!(HAND_PRAY, "hand_pray", Gestures, ["maos_unidas", "please", "13_maos_unidas"]);
icon!(HAND_RAISED, "hand_raised", Gestures, ["mao_aberta", "raised_hand", "06_mao_aberta"]);
icon!(HAND_ROCK, "hand_rock", Gestures, ["rock", "22_rock"]);
icon!(HAND_THUMBS_DOWN, "hand_thumbs_down", Gestures, ["negativo", "dislike", "thumbs_down", "02_negativo"]);
icon!(HAND_THUMBS_UP, "hand_thumbs_up", Gestures, ["positivo", "like", "thumbs_up", "01_positivo"]);
icon!(HAND_VULCAN, "hand_vulcan", Gestures, ["saudacao_vulcana", "21_saudacao_vulcana"]);
icon!(HAND_WRITING, "hand_writing", Gestures, ["escrevendo", "24_escrevendo"]);
icon!(HARD_DRIVE, "hard_drive", Hardware, ["hdd", "51_disco_rigido"]);
icon!(HEADPHONES, "headphones", Media, ["39_fone_de_ouvido"]);
icon!(HEARING, "hearing", People, []);
icon!(HELP, "help", Status, ["question"]);
icon!(HOME, "home", Navigation, []);
icon!(HTML5, "html5", Technologies, ["04_09_html5"]);
icon!(HTML_FILE, "html_file", FileTypes, ["html", "htm"]);
icon!(ICO_FILE, "ico_file", FileTypes, ["ico"]);
icon!(IMG_FILE, "img_file", FileTypes, ["img", "disk_image_file"]);
icon!(INFO, "info", Status, ["information"]);
icon!(INSTAGRAM, "instagram", Social, ["02_instagram"]);
icon!(INTERNET_DOWNLOAD, "internet_download", Network, ["13_download_internet"]);
icon!(INTERNET_UPLOAD, "internet_upload", Network, ["14_upload_internet"]);
icon!(IPA_FILE, "ipa_file", FileTypes, ["ipa"]);
icon!(ISO_FILE, "iso_file", FileTypes, ["iso"]);
icon!(JAVA, "java", Technologies, ["04_01_java"]);
icon!(JAVA_FILE, "java_file", FileTypes, []);
icon!(JAVASCRIPT, "javascript", Technologies, ["js", "04_07_javascript"]);
icon!(JAVASCRIPT_FILE, "javascript_file", FileTypes, ["js_file"]);
icon!(JAVASCRIPT_HEX, "javascript_hex", Technologies, ["04_06_javascript_hex"]);
icon!(JENKINS, "jenkins", Technologies, ["03_03_jenkins"]);
icon!(JOYSTICK, "joystick", Entertainment, []);
icon!(JPEG_FILE, "jpeg_file", FileTypes, ["jpeg"]);
icon!(JPG_FILE, "jpg_file", FileTypes, ["jpg"]);
icon!(JSON_FILE, "json_file", FileTypes, ["json", "08_09_json"]);
icon!(JSX_FILE, "jsx_file", FileTypes, ["jsx"]);
icon!(KAFKA, "kafka", Technologies, ["05_11_kafka"]);
icon!(KAKAOTALK, "kakaotalk", Social, ["kakao_talk"]);
icon!(KEY, "key", Security, ["06_06_chave"]);
icon!(KEYBOARD, "keyboard", Hardware, []);
icon!(KEYS, "keys", Security, ["65_chaves_de_acesso"]);
icon!(KICK, "kick", Social, []);
icon!(KUAISHOU_KWAI, "kuaishou_kwai", Social, ["kwai", "kuaishou"]);
icon!(KUBERNETES, "kubernetes", Technologies, ["k8s", "03_05_kubernetes"]);
icon!(LAPTOP, "laptop", Devices, ["notebook", "02_notebook", "01_02_notebook"]);
icon!(LEMON8, "lemon8", Social, []);
icon!(LIGHTBULB, "lightbulb", Office, ["idea", "idea_lightbulb"]);
icon!(LIKEE, "likee", Social, []);
icon!(LINE, "line", Social, ["36_line"]);
icon!(LINE_CHART, "line_chart", Data, ["chart"]);
icon!(LINK, "link", Actions, []);
icon!(LINKEDIN, "linkedin", Social, ["08_linkedin"]);
icon!(LINUX, "linux", Technologies, ["03_10_linux"]);
icon!(LOCK, "lock", Security, ["locked", "padlock", "06_07_cadeado"]);
icon!(M4A_FILE, "m4a_file", FileTypes, ["m4a"]);
icon!(MAIL, "mail", Communication, ["envelope"]);
icon!(MAP_PIN, "map_pin", Navigation, ["location", "map"]);
icon!(MASTODON, "mastodon", Social, ["32_mastodon"]);
icon!(MEDAL, "medal", Entertainment, ["award"]);
icon!(MEDIUM, "medium", Social, ["24_medium"]);
icon!(MESSENGER, "messenger", Social, ["26_messenger", "facebook_messenger"]);
icon!(MICROPHONE, "microphone", Media, ["mic", "40_microfone"]);
icon!(MICROSD_CARD, "microsd_card", Hardware, ["54_cartao_microsd"]);
icon!(MICROSOFT_TEAMS, "microsoft_teams", Social, ["30_microsoft_teams"]);
icon!(MKV_FILE, "mkv_file", FileTypes, ["mkv"]);
icon!(MODULES, "modules", Development, ["blocks", "02_02_modulos"]);
icon!(MONGODB, "mongodb", Technologies, ["05_04_mongodb"]);
icon!(MONITOR, "monitor", Devices, ["display", "screen"]);
icon!(MONITORING, "monitoring", Development, ["07_07_monitoramento"]);
icon!(MOUSE, "mouse", Hardware, []);
icon!(MOV_FILE, "mov_file", FileTypes, ["mov"]);
icon!(MP3_FILE, "mp3_file", FileTypes, ["mp3"]);
icon!(MP4_FILE, "mp4_file", FileTypes, ["mp4"]);
icon!(MSI_FILE, "msi_file", FileTypes, ["msi", "installer_file"]);
icon!(MUSIC, "music", Media, ["37_musica"]);
icon!(MUSIC_KEYBOARD, "music_keyboard", Media, ["piano"]);
icon!(MYSPACE, "myspace", Social, ["38_myspace"]);
icon!(MYSQL, "mysql", Technologies, ["05_06_mysql"]);
icon!(NETWORK, "network", Network, ["local_network", "lan", "07_01_rede_local"]);
icon!(NETWORK_SWITCH, "network_switch", Network, ["07_03_switch"]);
icon!(NEXTDOOR, "nextdoor", Social, []);
icon!(NOTEPAD, "notepad", Office, ["26_bloco_de_notas"]);
icon!(NOTIFICATION_BELL, "notification_bell", Communication, ["bell", "notification", "23_notificacao_sino"]);
icon!(ODNOKLASSNIKI, "odnoklassniki", Social, ["ok_ru"]);
icon!(OGG_FILE, "ogg_file", FileTypes, ["ogg"]);
icon!(OPENSHIFT, "openshift", Technologies, ["03_06_openshift"]);
icon!(OPTICAL_DISC, "optical_disc", Media, ["cd", "dvd", "41_disco_optico"]);
icon!(OPTIONS, "options", Actions, ["sliders", "preferences"]);
icon!(ORACLE, "oracle", Technologies, ["05_08_oracle"]);
icon!(PACKAGE, "package", Commerce, ["box", "46_pacote_fechado"]);
icon!(PACKAGE_OPEN, "package_open", Commerce, ["box_open", "47_pacote_aberto"]);
icon!(PACKAGE_SEARCH, "package_search", Commerce, ["tracking"]);
icon!(PAINT_BRUSH, "paint_brush", Design, ["brush"]);
icon!(PALETTE, "palette", Design, ["colors"]);
icon!(PDF_FILE, "pdf_file", FileTypes, ["pdf"]);
icon!(PHONE_CALL, "phone_call", Communication, ["call"]);
icon!(PHOTO_GALLERY, "photo_gallery", Media, ["gallery", "32_galeria_de_fotos"]);
icon!(PHP_FILE, "php_file", FileTypes, ["php"]);
icon!(PICTURE, "picture", Media, ["image", "photo", "31_foto_paisagem"]);
icon!(PIE_CHART_GROWTH, "pie_chart_growth", Data, ["pie_chart", "pie_chart_up"]);
icon!(PIE_CHART_REPORT, "pie_chart_report", Data, ["30_relatorio_pizza"]);
icon!(PIN, "pin", Office, ["pushpin", "24_fixar_alfinete"]);
icon!(PINTEREST, "pinterest", Social, ["14_pinterest"]);
icon!(PLAY_DISC, "play_disc", Media, ["43_reproduzir_disco"]);
icon!(PLUG, "plug", Hardware, ["power_plug", "06_05_plugue"]);
icon!(PNG_FILE, "png_file", FileTypes, ["png"]);
icon!(POPCORN, "popcorn", Entertainment, []);
icon!(POWER, "power", Actions, ["shutdown"]);
icon!(PPT_FILE, "ppt_file", FileTypes, ["ppt"]);
icon!(PPTX_FILE, "pptx_file", FileTypes, ["pptx", "powerpoint_file"]);
icon!(PRESENTATION, "presentation", Office, ["slides", "presentation_board"]);
icon!(PRINTER, "printer", Devices, ["print", "55_impressora"]);
icon!(PSD_FILE, "psd_file", FileTypes, ["psd", "photoshop_file"]);
icon!(PUZZLE, "puzzle", Development, ["plugin", "extension", "puzzle_piece"]);
icon!(PY_FILE, "py_file", FileTypes, ["py", "python_file"]);
icon!(PYTHON, "python", Technologies, ["04_02_python"]);
icon!(QQ, "qq", Social, []);
icon!(QR_CODE, "qr_code", Commerce, ["qr", "58_codigo_qr"]);
icon!(QUORA, "quora", Social, []);
icon!(RAR_FILE, "rar_file", FileTypes, ["rar"]);
icon!(RAW_FILE, "raw_file", FileTypes, ["raw"]);
icon!(REACT, "react", Technologies, ["05_01_react"]);
icon!(RECORD_DISC, "record_disc", Media, ["record_audio", "44_gravar_audio"]);
icon!(RECYCLE, "recycle", Actions, ["recycle_bin"]);
icon!(REDDIT, "reddit", Social, ["10_reddit"]);
icon!(REDIS, "redis", Technologies, ["05_09_redis"]);
icon!(REFRESH, "refresh", Actions, ["reload"]);
icon!(REMOVE, "remove", Actions, ["minus"]);
icon!(REST_API, "rest_api", Development, ["braces", "06_02_rest_api"]);
icon!(ROBOT, "robot", Development, ["bot", "automation", "29_robo"]);
icon!(ROUTER, "router", Network, ["07_02_roteador"]);
icon!(RS_FILE, "rs_file", FileTypes, ["rs", "rust_file"]);
icon!(RSS, "rss", Communication, ["feed"]);
icon!(RULER, "ruler", Design, ["measure"]);
icon!(RUMBLE, "rumble", Social, []);
icon!(RUN, "run", Development, ["execute", "03_08_execucao"]);
icon!(SAVE, "save", Actions, ["floppy"]);
icon!(SCANNER, "scanner", Devices, ["56_scanner"]);
icon!(SD_CARD, "sd_card", Hardware, ["53_cartao_sd"]);
icon!(SEARCH, "search", Actions, ["find", "magnifier", "07_08_pesquisa"]);
icon!(SERVER, "server", Network, ["02_05_servidor"]);
icon!(SERVER_RACK, "server_rack", Network, ["04_servidor_rack", "02_06_cluster_servidores"]);
icon!(SETTINGS, "settings", Actions, ["gear", "configuration", "59_engrenagem", "01_08_configuracoes"]);
icon!(SH_FILE, "sh_file", FileTypes, ["sh", "shell_script"]);
icon!(SHIELD, "shield", Security, ["06_09_escudo"]);
icon!(SHIELD_CHECK, "shield_check", Security, ["protection_on", "security", "61_protecao_ativa", "06_08_seguranca"]);
icon!(SHIELD_ERROR, "shield_error", Security, ["protection_off", "62_protecao_bloqueada"]);
icon!(SHIELD_LOCK, "shield_lock", Security, ["78_escudo_bloqueado"]);
icon!(SHOPPING_BAG, "shopping_bag", Commerce, []);
icon!(SHOPPING_CART, "shopping_cart", Commerce, ["cart"]);
icon!(SIGNAL, "signal", Social, ["34_signal"]);
icon!(SKYPE, "skype", Social, ["27_skype"]);
icon!(SMARTPHONE, "smartphone", Devices, ["phone", "mobile", "08_03_smartphone"]);
icon!(SNAPCHAT, "snapchat", Social, ["13_snapchat"]);
icon!(SOUNDCLOUD, "soundcloud", Social, ["16_soundcloud"]);
icon!(SPEAKER, "speaker", Media, ["volume", "sound", "38_alto_falante"]);
icon!(SPOTIFY, "spotify", Social, ["15_spotify"]);
icon!(SPREADSHEET, "spreadsheet", Data, ["table", "sheet"]);
icon!(SPRING, "spring", Technologies, ["05_03_spring"]);
icon!(SQL_FILE, "sql_file", FileTypes, ["sql"]);
icon!(SSD, "ssd", Hardware, ["52_unidade_ssd"]);
icon!(STACK_OVERFLOW, "stack_overflow", Social, ["23_stack_overflow"]);
icon!(STAMP, "stamp", Office, ["approve"]);
icon!(STICKY_NOTE, "sticky_note", Office, ["25_post_it"]);
icon!(SVG_FILE, "svg_file", FileTypes, ["svg"]);
icon!(SYNC, "sync", Actions, ["synchronize", "79_sincronizar"]);
icon!(SYNC_WINDOW, "sync_window", Development, []);
icon!(TABLET, "tablet", Devices, ["08_04_tablet"]);
icon!(TAR_FILE, "tar_file", FileTypes, ["tar"]);
icon!(TEAM_WINDOW, "team_window", People, ["meeting"]);
icon!(TELEGRAM, "telegram", Social, ["06_telegram"]);
icon!(TERMINAL, "terminal", Development, ["console", "shell", "01_07_terminal"]);
icon!(THREADS, "threads", Social, ["09_threads"]);
icon!(THREADS_ALTERNATE, "threads_alternate", Social, ["31_threads_alternate"]);
icon!(TIFF_FILE, "tiff_file", FileTypes, ["tiff", "tif"]);
icon!(TIKTOK, "tiktok", Social, ["07_tiktok"]);
icon!(TOOLS, "tools", Office, ["maintenance", "wrench", "60_manutencao", "01_09_ferramentas"]);
icon!(TROPHY, "trophy", Entertainment, ["winner"]);
icon!(TS_FILE, "ts_file", FileTypes, ["typescript_file"]);
icon!(TSX_FILE, "tsx_file", FileTypes, ["tsx"]);
icon!(TUMBLR, "tumblr", Social, []);
icon!(TWITCH, "twitch", Social, ["12_twitch"]);
icon!(TYPESCRIPT, "typescript", Technologies, ["ts", "04_08_typescript"]);
icon!(UI_DESIGN, "ui_design", Design, ["08_01_design_interface"]);
icon!(UNLOCK, "unlock", Security, ["unlocked"]);
icon!(UP, "up", Actions, ["arrow_up"]);
icon!(UPLOAD, "upload", Actions, []);
icon!(UPLOAD_WINDOW, "upload_window", Development, []);
icon!(USB_DRIVE, "usb_drive", Hardware, ["pendrive", "flash_drive", "50_pendrive"]);
icon!(USER, "user", People, ["user_male", "person", "profile", "66_perfil_masculino"]);
icon!(USER_FEMALE, "user_female", People, ["67_perfil_feminino"]);
icon!(USERS, "users", People, ["user_group", "group", "team", "68_grupo_de_usuarios"]);
icon!(VIBER, "viber", Social, ["37_viber"]);
icon!(VIDEO_CAMERA, "video_camera", Media, ["video", "camcorder", "34_filmadora"]);
icon!(VIMEO, "vimeo", Social, ["17_vimeo"]);
icon!(VK_VKONTAKTE, "vk_vkontakte", Social, ["vk", "vkontakte"]);
icon!(VR_HEADSET, "vr_headset", Entertainment, ["vr"]);
icon!(VUE, "vue", Technologies, ["05_02_vue"]);
icon!(WARNING, "warning", Status, ["alert"]);
icon!(WAV_FILE, "wav_file", FileTypes, ["wav"]);
icon!(WEB_APP, "web_app", Development, ["08_02_web_app"]);
icon!(WEB_BROWSER, "web_browser", Network, ["browser", "11_navegador_internet"]);
icon!(WEB_LINK, "web_link", Network, ["15_link_web"]);
icon!(WEB_SEARCH, "web_search", Network, ["12_pesquisa_web"]);
icon!(WEB_WINDOW, "web_window", Development, ["website"]);
icon!(WEBCAM, "webcam", Hardware, []);
icon!(WEBM_FILE, "webm_file", FileTypes, ["webm"]);
icon!(WEBP_FILE, "webp_file", FileTypes, ["webp"]);
icon!(WECHAT, "wechat", Social, ["35_wechat"]);
icon!(WEIBO, "weibo", Social, []);
icon!(WHATSAPP, "whatsapp", Social, ["05_whatsapp"]);
icon!(WHATSAPP_BUSINESS, "whatsapp_business", Social, ["33_whatsapp_business"]);
icon!(WIFI, "wifi", Network, ["wireless", "10_wifi", "07_05_wifi"]);
icon!(WINDOW_STACK, "window_stack", Development, ["windows_stack"]);
icon!(WINDOWS, "windows", Technologies, ["03_11_windows"]);
icon!(X_TWITTER, "x_twitter", Social, ["04_x_twitter", "twitter", "x"]);
icon!(XIAOHONGSHU_REDNOTE, "xiaohongshu_rednote", Social, ["xiaohongshu", "rednote"]);
icon!(XLS_FILE, "xls_file", FileTypes, ["xls"]);
icon!(XLSX_FILE, "xlsx_file", FileTypes, ["xlsx", "excel_file"]);
icon!(XML_FILE, "xml_file", FileTypes, ["xml"]);
icon!(YAML_FILE, "yaml_file", FileTypes, ["yaml", "yml"]);
icon!(YOUTUBE, "youtube", Social, ["01_youtube"]);
icon!(YOUTUBE_SHORTS, "youtube_shorts", Social, ["25_youtube_shorts"]);
icon!(ZHIHU, "zhihu", Social, []);
icon!(ZIP_FILE, "zip_file", FileTypes, ["zip"]);
icon!(ZIP_FOLDER, "zip_folder", Files, ["49_pasta_zip"]);
icon!(ZOOM, "zoom", Social, ["28_zoom"]);
icon!(ZOOM_IN, "zoom_in", Actions, []);
icon!(ZOOM_OUT, "zoom_out", Actions, []);

/// All embedded icons alphabetically by name.
pub(super) static ALL: [&IconAsset; 392] = [
    &SEVEN_ZIP_FILE,
    &AAC_FILE,
    &ACCESSIBILITY,
    &ADD,
    &AI_FILE,
    &ALARM_CLOCK,
    &ANGULAR,
    &API_HUB,
    &APK_FILE,
    &ARCHIVE,
    &AVI_FILE,
    &AZURE_DEVOPS,
    &BACK,
    &BAIDU_TIEBA,
    &BAR_CHART_REPORT,
    &BARCODE,
    &BARCODE_SCANNER,
    &BAT_FILE,
    &BATTERY,
    &BATTERY_CHARGING,
    &BEHANCE,
    &BEREAL,
    &BILIBILI,
    &BLOCKED,
    &BLUESKY,
    &BMP_FILE,
    &BRIEFCASE,
    &BUG,
    &BURN_DISC,
    &C_FILE,
    &CALCULATOR,
    &CALENDAR,
    &CAMERA,
    &CHAT,
    &CHECK,
    &CHECKLIST_DONE,
    &CHECKLIST_PENDING,
    &CLAPPERBOARD,
    &CLIPBOARD,
    &CLOCK,
    &CLOSE,
    &CLOUD,
    &CLOUD_DOWNLOAD,
    &CLOUD_LOCK,
    &CLOUD_SETTINGS,
    &CLOUD_SYNC,
    &CLOUD_TRANSFER,
    &CLOUD_UPLOAD,
    &CODE_EDITOR,
    &CODE_FILE,
    &CODE_WINDOW,
    &COMPASS,
    &COMPRESSED_FILES,
    &COMPUTER,
    &CONTACTS,
    &COPY,
    &CPP,
    &CPP_FILE,
    &CPU,
    &CREDIT_CARD,
    &CS_FILE,
    &CSHARP,
    &CSS3,
    &CSS_FILE,
    &CSV_FILE,
    &CUT,
    &DATABASE,
    &DATABASE_SEARCH,
    &DATABASE_SYNC,
    &DB_FILE,
    &DELETE,
    &DELIVERY_TRUCK,
    &DEVELOPER,
    &DEVOPS,
    &DISC_DOWNLOAD,
    &DISCORD,
    &DLL_FILE,
    &DOC_FILE,
    &DOCKER,
    &DOCX_FILE,
    &DOTNET,
    &DOUYIN,
    &DOWN,
    &DOWNLOAD,
    &DOWNLOAD_WINDOW,
    &DRIBBBLE,
    &DUAL_MONITORS,
    &EDIT,
    &EDIT_NOTE,
    &ELASTICSEARCH,
    &EMAIL,
    &EMAIL_RECEIVE,
    &EMAIL_SEND,
    &EMOJI_GRIN,
    &EMOJI_HAND_OVER_MOUTH,
    &EMOJI_HUG,
    &EMOJI_JOY,
    &EMOJI_KISS,
    &EMOJI_MIND_BLOWN,
    &EMOJI_NAUSEATED,
    &EMOJI_NERD,
    &EMOJI_NERVOUS,
    &EMOJI_PARTY,
    &EMOJI_POOP,
    &EMOJI_RELIEVED,
    &EMOJI_SCREAM,
    &EMOJI_SMILE,
    &EMOJI_STAR_STRUCK,
    &EMOJI_SURPRISED,
    &EMOJI_SUSPICIOUS,
    &EMOJI_THINKING,
    &EMOJI_TONGUE,
    &EMOJI_WINK,
    &EPS_FILE,
    &EXE_FILE,
    &EYEDROPPER,
    &FACEBOOK,
    &FAVORITE,
    &FAVORITE_ADD,
    &FAVORITE_REMOVE,
    &FILE,
    &FILM_REEL,
    &FIREWALL,
    &FLAC_FILE,
    &FLAG_BLUE,
    &FLAG_CHECK,
    &FLAG_ERROR,
    &FLAG_GREEN,
    &FLAG_RED,
    &FLICKR,
    &FLOWCHART,
    &FOLDER,
    &FOLDER_OPEN,
    &FORWARD,
    &FOUNTAIN_PEN,
    &GAMEPAD,
    &GIF_FILE,
    &GIFT,
    &GIT,
    &GITHUB,
    &GITHUB_SOCIAL,
    &GITLAB,
    &GITLAB_SOCIAL,
    &GLOBAL_NETWORK,
    &GLOBE,
    &GO_FILE,
    &GOOGLE_MEET,
    &GPS_NAVIGATION,
    &GZ_FILE,
    &H_FILE,
    &HAMMER,
    &HAND_BACKHAND_POINT,
    &HAND_CALL_ME,
    &HAND_CLAP,
    &HAND_FINGERS_CROSSED,
    &HAND_FIST,
    &HAND_FLEXED_BICEPS,
    &HAND_LOVE_YOU,
    &HAND_OK,
    &HAND_OPEN_HANDS,
    &HAND_PALMS_UP,
    &HAND_PEACE,
    &HAND_PINCH,
    &HAND_PINCHED_FINGERS,
    &HAND_POINT_LEFT,
    &HAND_POINT_UP,
    &HAND_PRAY,
    &HAND_RAISED,
    &HAND_ROCK,
    &HAND_THUMBS_DOWN,
    &HAND_THUMBS_UP,
    &HAND_VULCAN,
    &HAND_WRITING,
    &HARD_DRIVE,
    &HEADPHONES,
    &HEARING,
    &HELP,
    &HOME,
    &HTML5,
    &HTML_FILE,
    &ICO_FILE,
    &IMG_FILE,
    &INFO,
    &INSTAGRAM,
    &INTERNET_DOWNLOAD,
    &INTERNET_UPLOAD,
    &IPA_FILE,
    &ISO_FILE,
    &JAVA,
    &JAVA_FILE,
    &JAVASCRIPT,
    &JAVASCRIPT_FILE,
    &JAVASCRIPT_HEX,
    &JENKINS,
    &JOYSTICK,
    &JPEG_FILE,
    &JPG_FILE,
    &JSON_FILE,
    &JSX_FILE,
    &KAFKA,
    &KAKAOTALK,
    &KEY,
    &KEYBOARD,
    &KEYS,
    &KICK,
    &KUAISHOU_KWAI,
    &KUBERNETES,
    &LAPTOP,
    &LEMON8,
    &LIGHTBULB,
    &LIKEE,
    &LINE,
    &LINE_CHART,
    &LINK,
    &LINKEDIN,
    &LINUX,
    &LOCK,
    &M4A_FILE,
    &MAIL,
    &MAP_PIN,
    &MASTODON,
    &MEDAL,
    &MEDIUM,
    &MESSENGER,
    &MICROPHONE,
    &MICROSD_CARD,
    &MICROSOFT_TEAMS,
    &MKV_FILE,
    &MODULES,
    &MONGODB,
    &MONITOR,
    &MONITORING,
    &MOUSE,
    &MOV_FILE,
    &MP3_FILE,
    &MP4_FILE,
    &MSI_FILE,
    &MUSIC,
    &MUSIC_KEYBOARD,
    &MYSPACE,
    &MYSQL,
    &NETWORK,
    &NETWORK_SWITCH,
    &NEXTDOOR,
    &NOTEPAD,
    &NOTIFICATION_BELL,
    &ODNOKLASSNIKI,
    &OGG_FILE,
    &OPENSHIFT,
    &OPTICAL_DISC,
    &OPTIONS,
    &ORACLE,
    &PACKAGE,
    &PACKAGE_OPEN,
    &PACKAGE_SEARCH,
    &PAINT_BRUSH,
    &PALETTE,
    &PDF_FILE,
    &PHONE_CALL,
    &PHOTO_GALLERY,
    &PHP_FILE,
    &PICTURE,
    &PIE_CHART_GROWTH,
    &PIE_CHART_REPORT,
    &PIN,
    &PINTEREST,
    &PLAY_DISC,
    &PLUG,
    &PNG_FILE,
    &POPCORN,
    &POWER,
    &PPT_FILE,
    &PPTX_FILE,
    &PRESENTATION,
    &PRINTER,
    &PSD_FILE,
    &PUZZLE,
    &PY_FILE,
    &PYTHON,
    &QQ,
    &QR_CODE,
    &QUORA,
    &RAR_FILE,
    &RAW_FILE,
    &REACT,
    &RECORD_DISC,
    &RECYCLE,
    &REDDIT,
    &REDIS,
    &REFRESH,
    &REMOVE,
    &REST_API,
    &ROBOT,
    &ROUTER,
    &RS_FILE,
    &RSS,
    &RULER,
    &RUMBLE,
    &RUN,
    &SAVE,
    &SCANNER,
    &SD_CARD,
    &SEARCH,
    &SERVER,
    &SERVER_RACK,
    &SETTINGS,
    &SH_FILE,
    &SHIELD,
    &SHIELD_CHECK,
    &SHIELD_ERROR,
    &SHIELD_LOCK,
    &SHOPPING_BAG,
    &SHOPPING_CART,
    &SIGNAL,
    &SKYPE,
    &SMARTPHONE,
    &SNAPCHAT,
    &SOUNDCLOUD,
    &SPEAKER,
    &SPOTIFY,
    &SPREADSHEET,
    &SPRING,
    &SQL_FILE,
    &SSD,
    &STACK_OVERFLOW,
    &STAMP,
    &STICKY_NOTE,
    &SVG_FILE,
    &SYNC,
    &SYNC_WINDOW,
    &TABLET,
    &TAR_FILE,
    &TEAM_WINDOW,
    &TELEGRAM,
    &TERMINAL,
    &THREADS,
    &THREADS_ALTERNATE,
    &TIFF_FILE,
    &TIKTOK,
    &TOOLS,
    &TROPHY,
    &TS_FILE,
    &TSX_FILE,
    &TUMBLR,
    &TWITCH,
    &TYPESCRIPT,
    &UI_DESIGN,
    &UNLOCK,
    &UP,
    &UPLOAD,
    &UPLOAD_WINDOW,
    &USB_DRIVE,
    &USER,
    &USER_FEMALE,
    &USERS,
    &VIBER,
    &VIDEO_CAMERA,
    &VIMEO,
    &VK_VKONTAKTE,
    &VR_HEADSET,
    &VUE,
    &WARNING,
    &WAV_FILE,
    &WEB_APP,
    &WEB_BROWSER,
    &WEB_LINK,
    &WEB_SEARCH,
    &WEB_WINDOW,
    &WEBCAM,
    &WEBM_FILE,
    &WEBP_FILE,
    &WECHAT,
    &WEIBO,
    &WHATSAPP,
    &WHATSAPP_BUSINESS,
    &WIFI,
    &WINDOW_STACK,
    &WINDOWS,
    &X_TWITTER,
    &XIAOHONGSHU_REDNOTE,
    &XLS_FILE,
    &XLSX_FILE,
    &XML_FILE,
    &YAML_FILE,
    &YOUTUBE,
    &YOUTUBE_SHORTS,
    &ZHIHU,
    &ZIP_FILE,
    &ZIP_FOLDER,
    &ZOOM,
    &ZOOM_IN,
    &ZOOM_OUT,
];

/// Lowercase extension without dot → file type icon, alphabetically.
pub(super) static EXTENSIONS: [(&str, &IconAsset); 92] = [
    ("7z", &SEVEN_ZIP_FILE),
    ("aac", &AAC_FILE),
    ("ai", &AI_FILE),
    ("apk", &APK_FILE),
    ("arw", &RAW_FILE),
    ("avi", &AVI_FILE),
    ("bash", &SH_FILE),
    ("bat", &BAT_FILE),
    ("bmp", &BMP_FILE),
    ("c", &C_FILE),
    ("c++", &CPP_FILE),
    ("cc", &CPP_FILE),
    ("cjs", &JAVASCRIPT_FILE),
    ("cmd", &BAT_FILE),
    ("cpp", &CPP_FILE),
    ("cr2", &RAW_FILE),
    ("cs", &CS_FILE),
    ("css", &CSS_FILE),
    ("csv", &CSV_FILE),
    ("cxx", &CPP_FILE),
    ("db", &DB_FILE),
    ("dll", &DLL_FILE),
    ("dng", &RAW_FILE),
    ("doc", &DOC_FILE),
    ("docx", &DOCX_FILE),
    ("eps", &EPS_FILE),
    ("exe", &EXE_FILE),
    ("flac", &FLAC_FILE),
    ("gif", &GIF_FILE),
    ("go", &GO_FILE),
    ("gz", &GZ_FILE),
    ("h", &H_FILE),
    ("hh", &H_FILE),
    ("hpp", &H_FILE),
    ("htm", &HTML_FILE),
    ("html", &HTML_FILE),
    ("hxx", &H_FILE),
    ("ico", &ICO_FILE),
    ("img", &IMG_FILE),
    ("ipa", &IPA_FILE),
    ("iso", &ISO_FILE),
    ("java", &JAVA_FILE),
    ("jfif", &JPEG_FILE),
    ("jpe", &JPEG_FILE),
    ("jpeg", &JPEG_FILE),
    ("jpg", &JPG_FILE),
    ("js", &JAVASCRIPT_FILE),
    ("json", &JSON_FILE),
    ("jsx", &JSX_FILE),
    ("m4a", &M4A_FILE),
    ("m4v", &MP4_FILE),
    ("mjs", &JAVASCRIPT_FILE),
    ("mkv", &MKV_FILE),
    ("mov", &MOV_FILE),
    ("mp3", &MP3_FILE),
    ("mp4", &MP4_FILE),
    ("msi", &MSI_FILE),
    ("nef", &RAW_FILE),
    ("oga", &OGG_FILE),
    ("ogg", &OGG_FILE),
    ("pdf", &PDF_FILE),
    ("php", &PHP_FILE),
    ("png", &PNG_FILE),
    ("ppt", &PPT_FILE),
    ("pptx", &PPTX_FILE),
    ("psd", &PSD_FILE),
    ("py", &PY_FILE),
    ("pyw", &PY_FILE),
    ("rar", &RAR_FILE),
    ("raw", &RAW_FILE),
    ("rs", &RS_FILE),
    ("sh", &SH_FILE),
    ("sql", &SQL_FILE),
    ("sqlite", &DB_FILE),
    ("sqlite3", &DB_FILE),
    ("svg", &SVG_FILE),
    ("tar", &TAR_FILE),
    ("tgz", &GZ_FILE),
    ("tif", &TIFF_FILE),
    ("tiff", &TIFF_FILE),
    ("ts", &TS_FILE),
    ("tsx", &TSX_FILE),
    ("wav", &WAV_FILE),
    ("webm", &WEBM_FILE),
    ("webp", &WEBP_FILE),
    ("xls", &XLS_FILE),
    ("xlsx", &XLSX_FILE),
    ("xml", &XML_FILE),
    ("yaml", &YAML_FILE),
    ("yml", &YAML_FILE),
    ("zip", &ZIP_FILE),
    ("zsh", &SH_FILE),
];
