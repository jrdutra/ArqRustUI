//! ArqRustUI social icon bytes (part 8 of 8).
//!
//! Data-only crate: the icon bundle is split into parts of at most 9 MB to fit the crates.io limit. Use
//! `arqrustui::icons` instead (the `icons` feature is enabled by default), through the `arqrustui-icons` facade.
//!
//! Each constant is `(.ico, .png, .svg)`. ICO frames are 16, 24, 32, 48, 64, 128 and 256 px; `None` for PNG means the
//! PNG is the ICO's 256 px frame, stored only once.
#![no_std]
#![warn(missing_docs)]

/// An icon as `(.ico, .png, .svg)`.
type Bytes = (&'static [u8], Option<&'static [u8]>, Option<&'static [u8]>);

/// `messenger`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const MESSENGER: Bytes = (
    include_bytes!("../ico/messenger.ico"),
    Some(include_bytes!("../png/messenger.png")),
    Some(include_bytes!("../svg/messenger.svg")),
);
/// `microsoft_teams`: `.ico` original social icon, `.png` = 256 px frame.
pub const MICROSOFT_TEAMS: Bytes = (include_bytes!("../ico/microsoft_teams.ico"), None, None);
/// `myspace`: `.ico` original social icon, `.png` = 256 px frame.
pub const MYSPACE: Bytes = (include_bytes!("../ico/myspace.ico"), None, None);
/// `pinterest`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const PINTEREST: Bytes = (
    include_bytes!("../ico/pinterest.ico"),
    Some(include_bytes!("../png/pinterest.png")),
    Some(include_bytes!("../svg/pinterest.svg")),
);
/// `reddit`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const REDDIT: Bytes = (
    include_bytes!("../ico/reddit.ico"),
    Some(include_bytes!("../png/reddit.png")),
    Some(include_bytes!("../svg/reddit.svg")),
);
/// `signal`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const SIGNAL: Bytes = (
    include_bytes!("../ico/signal.ico"),
    Some(include_bytes!("../png/signal.png")),
    Some(include_bytes!("../svg/signal.svg")),
);
/// `skype`: `.ico` original social icon, `.png` = 256 px frame.
pub const SKYPE: Bytes = (include_bytes!("../ico/skype.ico"), None, None);
/// `snapchat`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const SNAPCHAT: Bytes = (
    include_bytes!("../ico/snapchat.ico"),
    Some(include_bytes!("../png/snapchat.png")),
    Some(include_bytes!("../svg/snapchat.svg")),
);
/// `soundcloud`: `.ico` original social icon, `.png` = 256 px frame.
pub const SOUNDCLOUD: Bytes = (include_bytes!("../ico/soundcloud.ico"), None, None);
/// `spotify`: `.ico` original social icon, `.png` = 256 px frame.
pub const SPOTIFY: Bytes = (include_bytes!("../ico/spotify.ico"), None, None);
/// `stack_overflow`: `.ico` original social icon, `.png` = 256 px frame.
pub const STACK_OVERFLOW: Bytes = (include_bytes!("../ico/stack_overflow.ico"), None, None);
/// `telegram`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const TELEGRAM: Bytes = (
    include_bytes!("../ico/telegram.ico"),
    Some(include_bytes!("../png/telegram.png")),
    Some(include_bytes!("../svg/telegram.svg")),
);
/// `threads_alternate`: `.ico` original social icon, `.png` = 256 px frame.
pub const THREADS_ALTERNATE: Bytes = (include_bytes!("../ico/threads_alternate.ico"), None, None);
/// `tiktok`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const TIKTOK: Bytes = (
    include_bytes!("../ico/tiktok.ico"),
    Some(include_bytes!("../png/tiktok.png")),
    Some(include_bytes!("../svg/tiktok.svg")),
);
/// `twitch`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const TWITCH: Bytes = (
    include_bytes!("../ico/twitch.ico"),
    Some(include_bytes!("../png/twitch.png")),
    Some(include_bytes!("../svg/twitch.svg")),
);
/// `viber`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const VIBER: Bytes = (
    include_bytes!("../ico/viber.ico"),
    Some(include_bytes!("../png/viber.png")),
    Some(include_bytes!("../svg/viber.svg")),
);
/// `vimeo`: `.ico` original social icon, `.png` = 256 px frame.
pub const VIMEO: Bytes = (include_bytes!("../ico/vimeo.ico"), None, None);
/// `wechat`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const WECHAT: Bytes = (
    include_bytes!("../ico/wechat.ico"),
    Some(include_bytes!("../png/wechat.png")),
    Some(include_bytes!("../svg/wechat.svg")),
);
/// `whatsapp`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const WHATSAPP: Bytes = (
    include_bytes!("../ico/whatsapp.ico"),
    Some(include_bytes!("../png/whatsapp.png")),
    Some(include_bytes!("../svg/whatsapp.svg")),
);
/// `whatsapp_business`: `.ico` original social icon, `.png` = 256 px frame.
pub const WHATSAPP_BUSINESS: Bytes = (include_bytes!("../ico/whatsapp_business.ico"), None, None);
/// `x_twitter`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const X_TWITTER: Bytes = (
    include_bytes!("../ico/x_twitter.ico"),
    Some(include_bytes!("../png/x_twitter.png")),
    Some(include_bytes!("../svg/x_twitter.svg")),
);
/// `youtube`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const YOUTUBE: Bytes = (
    include_bytes!("../ico/youtube.ico"),
    Some(include_bytes!("../png/youtube.png")),
    Some(include_bytes!("../svg/youtube.svg")),
);
/// `youtube_shorts`: `.ico` original social icon, `.png` = 256 px frame.
pub const YOUTUBE_SHORTS: Bytes = (include_bytes!("../ico/youtube_shorts.ico"), None, None);
/// `zoom`: `.ico` original social icon, `.png` = 256 px frame.
pub const ZOOM: Bytes = (include_bytes!("../ico/zoom.ico"), None, None);

/// `tumblr`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const TUMBLR: Bytes = (
    include_bytes!("../ico/tumblr.ico"),
    Some(include_bytes!("../png/tumblr.png")),
    Some(include_bytes!("../svg/tumblr.svg")),
);

/// `zhihu`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const ZHIHU: Bytes = (
    include_bytes!("../ico/zhihu.ico"),
    Some(include_bytes!("../png/zhihu.png")),
    Some(include_bytes!("../svg/zhihu.svg")),
);

/// `douyin`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const DOUYIN: Bytes = (
    include_bytes!("../ico/douyin.ico"),
    Some(include_bytes!("../png/douyin.png")),
    Some(include_bytes!("../svg/douyin.svg")),
);

/// `rumble`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const RUMBLE: Bytes = (
    include_bytes!("../ico/rumble.ico"),
    Some(include_bytes!("../png/rumble.png")),
    Some(include_bytes!("../svg/rumble.svg")),
);

/// `kick`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const KICK: Bytes = (
    include_bytes!("../ico/kick.ico"),
    Some(include_bytes!("../png/kick.png")),
    Some(include_bytes!("../svg/kick.svg")),
);

/// `kakaotalk`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const KAKAOTALK: Bytes = (
    include_bytes!("../ico/kakaotalk.ico"),
    Some(include_bytes!("../png/kakaotalk.png")),
    Some(include_bytes!("../svg/kakaotalk.svg")),
);

/// `nextdoor`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const NEXTDOOR: Bytes = (
    include_bytes!("../ico/nextdoor.ico"),
    Some(include_bytes!("../png/nextdoor.png")),
    Some(include_bytes!("../svg/nextdoor.svg")),
);

/// `qq`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const QQ: Bytes =
    (include_bytes!("../ico/qq.ico"), Some(include_bytes!("../png/qq.png")), Some(include_bytes!("../svg/qq.svg")));

/// `xiaohongshu_rednote`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see
/// THIRD_PARTY_NOTICES.md).
pub const XIAOHONGSHU_REDNOTE: Bytes = (
    include_bytes!("../ico/xiaohongshu_rednote.ico"),
    Some(include_bytes!("../png/xiaohongshu_rednote.png")),
    Some(include_bytes!("../svg/xiaohongshu_rednote.svg")),
);

/// `vk_vkontakte`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const VK_VKONTAKTE: Bytes = (
    include_bytes!("../ico/vk_vkontakte.ico"),
    Some(include_bytes!("../png/vk_vkontakte.png")),
    Some(include_bytes!("../svg/vk_vkontakte.svg")),
);

/// `bilibili`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const BILIBILI: Bytes = (
    include_bytes!("../ico/bilibili.ico"),
    Some(include_bytes!("../png/bilibili.png")),
    Some(include_bytes!("../svg/bilibili.svg")),
);

/// `quora`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const QUORA: Bytes = (
    include_bytes!("../ico/quora.ico"),
    Some(include_bytes!("../png/quora.png")),
    Some(include_bytes!("../svg/quora.svg")),
);

/// `lemon8`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const LEMON8: Bytes = (
    include_bytes!("../ico/lemon8.ico"),
    Some(include_bytes!("../png/lemon8.png")),
    Some(include_bytes!("../svg/lemon8.svg")),
);

/// `kuaishou_kwai`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const KUAISHOU_KWAI: Bytes = (
    include_bytes!("../ico/kuaishou_kwai.ico"),
    Some(include_bytes!("../png/kuaishou_kwai.png")),
    Some(include_bytes!("../svg/kuaishou_kwai.svg")),
);

/// `weibo`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const WEIBO: Bytes = (
    include_bytes!("../ico/weibo.ico"),
    Some(include_bytes!("../png/weibo.png")),
    Some(include_bytes!("../svg/weibo.svg")),
);

/// `likee`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const LIKEE: Bytes = (
    include_bytes!("../ico/likee.ico"),
    Some(include_bytes!("../png/likee.png")),
    Some(include_bytes!("../svg/likee.svg")),
);

/// `odnoklassniki`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const ODNOKLASSNIKI: Bytes = (
    include_bytes!("../ico/odnoklassniki.ico"),
    Some(include_bytes!("../png/odnoklassniki.png")),
    Some(include_bytes!("../svg/odnoklassniki.svg")),
);

/// `bereal`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const BEREAL: Bytes = (
    include_bytes!("../ico/bereal.ico"),
    Some(include_bytes!("../png/bereal.png")),
    Some(include_bytes!("../svg/bereal.svg")),
);
