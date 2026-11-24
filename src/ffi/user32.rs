//! Declarations for `user32.dll`: windows, messages, menus, input and clipboard.

use super::gdi32::LOGFONTW;
use super::types::*;
use std::ffi::c_void;

// ---- Mensagens ---------------------------------------------------------------
pub const WM_NULL: UINT = 0x0000;
pub const WM_CREATE: UINT = 0x0001;
pub const WM_DESTROY: UINT = 0x0002;
pub const WM_MOVE: UINT = 0x0003;
pub const WM_SIZE: UINT = 0x0005;
pub const WM_ACTIVATE: UINT = 0x0006;
pub const WM_SETFOCUS: UINT = 0x0007;
pub const WM_KILLFOCUS: UINT = 0x0008;
pub const WM_SETREDRAW: UINT = 0x000B;
pub const WM_SETTEXT: UINT = 0x000C;
pub const WM_GETTEXT: UINT = 0x000D;
pub const WM_GETTEXTLENGTH: UINT = 0x000E;
pub const WM_PAINT: UINT = 0x000F;
pub const WM_CLOSE: UINT = 0x0010;
pub const WM_QUIT: UINT = 0x0012;
pub const WM_ERASEBKGND: UINT = 0x0014;
pub const WM_SETTINGCHANGE: UINT = 0x001A;
pub const WM_SETCURSOR: UINT = 0x0020;
pub const WM_GETMINMAXINFO: UINT = 0x0024;
pub const WM_SETFONT: UINT = 0x0030;
pub const WM_GETFONT: UINT = 0x0031;
pub const WM_GETOBJECT: UINT = 0x003D;
pub const WM_GETDLGCODE: UINT = 0x0087;
pub const DLGC_WANTARROWS: LRESULT = 0x1;
pub const DLGC_WANTCHARS: LRESULT = 0x80;
pub const DLGC_WANTALLKEYS: LRESULT = 0x4;
pub const DLGC_HASSETSEL: LRESULT = 0x8;
pub const WM_DRAWITEM: UINT = 0x002B;
pub const WM_NOTIFY: UINT = 0x004E;
pub const WM_DISPLAYCHANGE: UINT = 0x007E;
pub const WM_CONTEXTMENU: UINT = 0x007B;
pub const WM_SETICON: UINT = 0x0080;
pub const WM_GETICON: UINT = 0x007F;
pub const WM_MEASUREITEM: UINT = 0x002C;
pub const LR_DEFAULTCOLOR: UINT = 0x0000;
pub const WM_NCDESTROY: UINT = 0x0082;
pub const WM_NCCALCSIZE: UINT = 0x0083;
pub const WM_NCHITTEST: UINT = 0x0084;
pub const WM_NCPAINT: UINT = 0x0085;
pub const WM_KEYDOWN: UINT = 0x0100;
pub const WM_KEYUP: UINT = 0x0101;
pub const WM_CHAR: UINT = 0x0102;
pub const WM_SYSKEYDOWN: UINT = 0x0104;
pub const WM_SYSKEYUP: UINT = 0x0105;
pub const WM_IME_STARTCOMPOSITION: UINT = 0x010D;
pub const WM_IME_ENDCOMPOSITION: UINT = 0x010E;
pub const WM_IME_COMPOSITION: UINT = 0x010F;
pub const WM_IME_SETCONTEXT: UINT = 0x0281;
pub const WM_GESTURE: UINT = 0x0119;
pub const GID_ZOOM: DWORD = 3;
pub const GID_PAN: DWORD = 4;
pub const GID_ROTATE: DWORD = 5;
pub const GF_BEGIN: DWORD = 0x1;
pub const GF_INERTIA: DWORD = 0x2;
pub const GF_END: DWORD = 0x4;
pub const GC_PAN_WITH_SINGLE_FINGER_VERTICALLY: DWORD = 0x2;
pub const GC_PAN_WITH_SINGLE_FINGER_HORIZONTALLY: DWORD = 0x4;
pub const GC_PAN_WITH_GUTTER: DWORD = 0x8;
pub const GC_PAN_WITH_INERTIA: DWORD = 0x10;

/// 16-bit coordinate point (`POINTS`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct POINTS {
    pub x: i16,
    pub y: i16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GESTURECONFIG {
    pub dwID: DWORD,
    pub dwWant: DWORD,
    pub dwBlock: DWORD,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GESTUREINFO {
    pub cbSize: UINT,
    pub dwFlags: DWORD,
    pub dwID: DWORD,
    pub hwndTarget: HWND,
    pub ptsLocation: POINTS,
    pub dwInstanceID: DWORD,
    pub dwSequenceID: DWORD,
    pub ullArguments: u64,
    pub cbExtraArgs: UINT,
}
pub const WM_IME_NOTIFY: UINT = 0x0282;
pub const WM_COMMAND: UINT = 0x0111;
pub const WM_SYSCOMMAND: UINT = 0x0112;
pub const WM_TIMER: UINT = 0x0113;
pub const WM_HSCROLL: UINT = 0x0114;
pub const WM_VSCROLL: UINT = 0x0115;
pub const WM_CHANGEUISTATE: UINT = 0x0127;
pub const WM_CTLCOLOREDIT: UINT = 0x0133;
pub const WM_CTLCOLORLISTBOX: UINT = 0x0134;
pub const WM_CTLCOLORBTN: UINT = 0x0135;
pub const WM_CTLCOLORSTATIC: UINT = 0x0138;
pub const WM_MOUSEMOVE: UINT = 0x0200;
pub const WM_LBUTTONDOWN: UINT = 0x0201;
pub const WM_LBUTTONUP: UINT = 0x0202;
pub const WM_LBUTTONDBLCLK: UINT = 0x0203;
pub const WM_RBUTTONDOWN: UINT = 0x0204;
pub const WM_RBUTTONUP: UINT = 0x0205;
pub const WM_MBUTTONDOWN: UINT = 0x0207;
pub const WM_MBUTTONUP: UINT = 0x0208;
pub const WM_RBUTTONDBLCLK: UINT = 0x0206;
pub const WM_MBUTTONDBLCLK: UINT = 0x0209;
pub const WM_MOUSEWHEEL: UINT = 0x020A;
pub const WM_DROPFILES: UINT = 0x0233;
pub const WM_POINTERUPDATE: UINT = 0x0245;
pub const WM_POINTERDOWN: UINT = 0x0246;
pub const WM_POINTERUP: UINT = 0x0247;
pub const WM_MOUSELEAVE: UINT = 0x02A3;
pub const WM_DPICHANGED: UINT = 0x02E0;
pub const WM_THEMECHANGED: UINT = 0x031A;
pub const WM_USER: UINT = 0x0400;
pub const WM_APP: UINT = 0x8000;
pub const DM_GETDEFID: UINT = WM_USER;

// ---- Estilos de janela ------------------------------------------------------
pub const WS_OVERLAPPED: u32 = 0x0000_0000;
pub const WS_POPUP: u32 = 0x8000_0000;
pub const WS_CHILD: u32 = 0x4000_0000;
pub const WS_MINIMIZE: u32 = 0x2000_0000;
pub const WS_VISIBLE: u32 = 0x1000_0000;
pub const WS_DISABLED: u32 = 0x0800_0000;
pub const WS_CLIPSIBLINGS: u32 = 0x0400_0000;
pub const WS_CLIPCHILDREN: u32 = 0x0200_0000;
pub const WS_MAXIMIZE: u32 = 0x0100_0000;
pub const WS_CAPTION: u32 = 0x00C0_0000;
pub const WS_BORDER: u32 = 0x0080_0000;
pub const WS_VSCROLL: u32 = 0x0020_0000;
pub const WS_HSCROLL: u32 = 0x0010_0000;
pub const WS_SYSMENU: u32 = 0x0008_0000;
pub const WS_THICKFRAME: u32 = 0x0004_0000;
pub const WS_GROUP: u32 = 0x0002_0000;
pub const WS_TABSTOP: u32 = 0x0001_0000;
pub const WS_MINIMIZEBOX: u32 = 0x0002_0000;
pub const WS_MAXIMIZEBOX: u32 = 0x0001_0000;
pub const WS_OVERLAPPEDWINDOW: u32 =
    WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX;

pub const WS_EX_DLGMODALFRAME: u32 = 0x0000_0001;
pub const WS_EX_TOPMOST: u32 = 0x0000_0008;
pub const WS_EX_ACCEPTFILES: u32 = 0x0000_0010;
pub const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
pub const WS_EX_CLIENTEDGE: u32 = 0x0000_0200;
pub const WS_EX_CONTROLPARENT: u32 = 0x0001_0000;
pub const WS_EX_LAYERED: u32 = 0x0008_0000;
pub const WS_EX_LAYOUTRTL: u32 = 0x0040_0000;
pub const WS_EX_COMPOSITED: u32 = 0x0200_0000;

pub const CS_VREDRAW: u32 = 0x0001;
pub const CS_HREDRAW: u32 = 0x0002;
pub const CS_DBLCLKS: u32 = 0x0008;
pub const CW_USEDEFAULT: i32 = 0x8000_0000_u32 as i32;
pub const HWND_MESSAGE: HWND = -3isize as HWND;
pub const HWND_TOP: HWND = std::ptr::null_mut();
pub const HWND_TOPMOST: HWND = -1isize as HWND;
pub const HWND_NOTOPMOST: HWND = -2isize as HWND;

pub const GWLP_WNDPROC: i32 = -4;
pub const GWL_STYLE: i32 = -16;
pub const GWL_EXSTYLE: i32 = -20;
pub const GA_ROOT: UINT = 2;

pub const SW_HIDE: i32 = 0;
pub const SW_SHOWNA: i32 = 8;
pub const SW_SHOWNORMAL: i32 = 1;
pub const SW_MAXIMIZE: i32 = 3;
pub const SW_SHOWNOACTIVATE: i32 = 4;
pub const SW_SHOW: i32 = 5;
pub const SW_MINIMIZE: i32 = 6;
pub const SW_RESTORE: i32 = 9;

pub const SWP_NOSIZE: UINT = 0x0001;
pub const SWP_NOMOVE: UINT = 0x0002;
pub const SWP_NOZORDER: UINT = 0x0004;
pub const SWP_NOACTIVATE: UINT = 0x0010;
pub const SWP_FRAMECHANGED: UINT = 0x0020;
pub const SWP_SHOWWINDOW: UINT = 0x0040;
pub const SWP_HIDEWINDOW: UINT = 0x0080;

pub const SIZE_MINIMIZED: WPARAM = 1;
pub const WA_INACTIVE: u16 = 0;
pub const HTCLIENT: u16 = 1;
pub const HTNOWHERE: u16 = 0;
pub const HTBORDER: u16 = 18;
pub const ICON_SMALL: WPARAM = 0;
pub const ICON_BIG: WPARAM = 1;
pub const LWA_ALPHA: DWORD = 0x2;
pub const TME_LEAVE: DWORD = 0x2;
pub const TME_NONCLIENT: DWORD = 0x10;
pub const HTMENU: u16 = 5;
pub const WM_NCMOUSEMOVE: UINT = 0x00A0;
pub const WM_NCLBUTTONDOWN: UINT = 0x00A1;
pub const WM_NCLBUTTONUP: UINT = 0x00A2;
pub const WM_NCLBUTTONDBLCLK: UINT = 0x00A3;
pub const WM_NCRBUTTONDOWN: UINT = 0x00A4;
pub const WM_NCRBUTTONUP: UINT = 0x00A5;
pub const WM_NCRBUTTONDBLCLK: UINT = 0x00A6;
pub const WM_NCMBUTTONDOWN: UINT = 0x00A7;
pub const WM_NCMBUTTONUP: UINT = 0x00A8;
pub const WM_NCMOUSELEAVE: UINT = 0x02A2;
pub const WM_NCACTIVATE: UINT = 0x0086;
pub const WM_CANCELMODE: UINT = 0x001F;
pub const WM_MOUSEACTIVATE: UINT = 0x0021;
pub const MA_NOACTIVATE: LRESULT = 3;
pub const WM_SYSCHAR: UINT = 0x0106;
pub const WM_ENTERSIZEMOVE: UINT = 0x0231;
pub const WS_EX_NOACTIVATE: u32 = 0x0800_0000;
pub const ULW_ALPHA: DWORD = 0x2;
pub const CS_DROPSHADOW: u32 = 0x0002_0000;
pub const SPI_GETMENUSHOWDELAY: UINT = 0x006A;
pub const COLOR_MENU: i32 = 4;
pub const SM_CXEDGE: i32 = 45;
pub const SM_CXDRAG: i32 = 68;
pub const SM_CYDRAG: i32 = 69;
pub const MK_LBUTTON: usize = 0x0001;
pub const MK_SHIFT: usize = 0x4;
pub const MK_CONTROL: usize = 0x8;
pub const MK_RBUTTON: usize = 0x0002;
pub const EVENT_SYSTEM_MENUSTART: DWORD = 0x0004;
pub const EVENT_SYSTEM_MENUEND: DWORD = 0x0005;
pub const EVENT_SYSTEM_MENUPOPUPSTART: DWORD = 0x0006;
pub const EVENT_SYSTEM_MENUPOPUPEND: DWORD = 0x0007;
pub const HTTRANSPARENT: LRESULT = -1;
pub const WS_EX_TRANSPARENT: u32 = 0x0000_0020;
pub const WM_CAPTURECHANGED: UINT = 0x0215;
pub const LB_ITEMFROMPOINT: UINT = 0x01A9;
pub const LB_GETITEMRECT: UINT = 0x0198;
pub const LB_SETTOPINDEX: UINT = 0x0197;
pub const LB_GETTOPINDEX: UINT = 0x018E;
pub const COLOR_ACTIVECAPTION: i32 = 2;
pub const COLOR_CAPTIONTEXT: i32 = 9;
pub const WM_PRINTCLIENT: UINT = 0x0318;
pub const COLOR_MENUTEXT: i32 = 7;
pub const COLOR_WINDOWFRAME: i32 = 6;
pub const COLOR_MENUBAR: i32 = 30;
pub const COLOR_MENUHILIGHT: i32 = 29;
pub const VK_RETURN: i32 = 0x0D;
pub const VK_ESCAPE: i32 = 0x1B;
pub const VK_END: i32 = 0x23;
pub const VK_HOME: i32 = 0x24;
pub const VK_LEFT: i32 = 0x25;
pub const VK_UP: i32 = 0x26;
pub const VK_RIGHT: i32 = 0x27;
pub const VK_DOWN: i32 = 0x28;
pub const VK_F10: i32 = 0x79;
pub const VK_SPACE: i32 = 0x20;
pub const VK_F4: i32 = 0x73;
pub const VK_APPS: i32 = 0x5D;
pub const WHEEL_DELTA: i32 = 120;
pub const UIS_CLEAR: u16 = 2;
pub const UISF_HIDEFOCUS: u16 = 0x1;
pub const WM_QUERYUISTATE: UINT = 0x0129;
pub const UISF_HIDEACCEL: u16 = 0x2;

// ---- Mensagens e estilos de controles clássicos --------------------------------
pub const BS_PUSHBUTTON: u32 = 0x0000;
pub const BS_LEFT: u32 = 0x0100;
pub const BS_RIGHT: u32 = 0x0200;
pub const BS_CENTER: u32 = 0x0300;
pub const BS_DEFPUSHBUTTON: u32 = 0x0001;
pub const BS_CHECKBOX: u32 = 0x0002;
pub const BS_AUTOCHECKBOX: u32 = 0x0003;
pub const BS_AUTO3STATE: u32 = 0x0006;
pub const BS_GROUPBOX: u32 = 0x0007;
pub const BS_AUTORADIOBUTTON: u32 = 0x0009;
pub const BS_OWNERDRAW: u32 = 0x000B;
pub const BS_TYPEMASK: u32 = 0x000F;
pub const BS_PUSHLIKE: u32 = 0x1000;
pub const BCM_GETIDEALSIZE: UINT = 0x1601;
pub const BCM_SETIMAGELIST: UINT = 0x1602;
pub const BCM_GETIMAGELIST: UINT = 0x1603;
pub const BUTTON_IMAGELIST_ALIGN_LEFT: UINT = 0;
pub const BUTTON_IMAGELIST_ALIGN_RIGHT: UINT = 1;
pub const BUTTON_IMAGELIST_ALIGN_TOP: UINT = 2;
pub const BUTTON_IMAGELIST_ALIGN_BOTTOM: UINT = 3;
pub const BUTTON_IMAGELIST_ALIGN_CENTER: UINT = 4;
pub const ODT_LISTBOX: UINT = 2;
pub const ODT_COMBOBOX: UINT = 3;
pub const ODT_STATIC: UINT = 5;
pub const ODS_COMBOBOXEDIT: UINT = 0x1000;
pub const ODS_NOFOCUSRECT: UINT = 0x0200;
pub const BM_SETSTYLE: UINT = 0x00F4;
pub const ODS_SELECTED: UINT = 0x0001;
pub const ODS_DISABLED: UINT = 0x0004;
pub const ODS_FOCUS: UINT = 0x0010;
pub const BS_NOTIFY: u32 = 0x4000;
pub const BM_GETCHECK: UINT = 0x00F0;
pub const BM_SETCHECK: UINT = 0x00F1;
pub const BM_CLICK: UINT = 0x00F5;
pub const BN_CLICKED: u16 = 0;
pub const BN_DBLCLK: u16 = 5;
pub const BST_UNCHECKED: usize = 0;
pub const BST_CHECKED: usize = 1;
pub const BST_INDETERMINATE: usize = 2;

pub const SS_LEFT: u32 = 0x0000;
pub const SS_CENTER: u32 = 0x0001;
pub const SS_RIGHT: u32 = 0x0002;
pub const SS_NOTIFY: u32 = 0x0100;
pub const SS_NOPREFIX: u32 = 0x0080;
pub const SS_CENTERIMAGE: u32 = 0x0200;
pub const SS_OWNERDRAW: u32 = 0x000D;
pub const SS_TYPEMASK: u32 = 0x001F;
pub const STN_CLICKED: u16 = 0;
pub const STN_DBLCLK: u16 = 1;

pub const ES_LEFT: u32 = 0x0000;
pub const ES_CENTER: u32 = 0x0001;
pub const ES_RIGHT: u32 = 0x0002;
pub const ES_MULTILINE: u32 = 0x0004;
pub const ES_PASSWORD: u32 = 0x0020;
pub const ES_AUTOVSCROLL: u32 = 0x0040;
pub const ES_AUTOHSCROLL: u32 = 0x0080;
pub const ES_NOHIDESEL: u32 = 0x0100;
pub const ES_READONLY: u32 = 0x0800;
pub const ES_WANTRETURN: u32 = 0x1000;
pub const ES_NUMBER: u32 = 0x2000;
pub const EN_CHANGE: u16 = 0x0300;
pub const EM_GETSEL: UINT = 0x00B0;
pub const EM_SETSEL: UINT = 0x00B1;
pub const EM_SCROLLCARET: UINT = 0x00B7;
pub const EM_GETMODIFY: UINT = 0x00B8;
pub const EM_SETMODIFY: UINT = 0x00B9;
pub const EM_GETLINECOUNT: UINT = 0x00BA;
pub const EM_REPLACESEL: UINT = 0x00C2;
pub const EM_LIMITTEXT: UINT = 0x00C5;
pub const EM_CANUNDO: UINT = 0x00C6;
pub const EM_UNDO: UINT = 0x00C7;
pub const EM_SETPASSWORDCHAR: UINT = 0x00CC;
pub const EM_SETREADONLY: UINT = 0x00CF;
pub const WM_ENABLE: UINT = 0x000A;
pub const EM_SETCUEBANNER: UINT = 0x1501;
pub const WM_CUT: UINT = 0x0300;
pub const WM_COPY: UINT = 0x0301;
pub const WM_PASTE: UINT = 0x0302;
pub const WM_CLEAR: UINT = 0x0303;
pub const WM_UNDO: UINT = 0x0304;

pub const CBS_DROPDOWN: u32 = 0x0002;
pub const CBS_DROPDOWNLIST: u32 = 0x0003;
pub const CBS_AUTOHSCROLL: u32 = 0x0040;
pub const CBS_SORT: u32 = 0x0100;
pub const CBS_HASSTRINGS: u32 = 0x0200;
pub const CBS_OWNERDRAWFIXED: u32 = 0x0010;
pub const CB_GETITEMDATA: UINT = 0x0150;
pub const CB_SETITEMDATA: UINT = 0x0151;
pub const CB_SETITEMHEIGHT: UINT = 0x0153;
pub const CB_GETITEMHEIGHT: UINT = 0x0154;
pub const CB_ADDSTRING: UINT = 0x0143;
pub const CB_DELETESTRING: UINT = 0x0144;
pub const CB_GETCOUNT: UINT = 0x0146;
pub const CB_GETCURSEL: UINT = 0x0147;
pub const CB_GETLBTEXT: UINT = 0x0148;
pub const CB_GETLBTEXTLEN: UINT = 0x0149;
pub const CB_INSERTSTRING: UINT = 0x014A;
pub const CB_RESETCONTENT: UINT = 0x014B;
pub const CB_SETCURSEL: UINT = 0x014E;
pub const CBN_SELCHANGE: u16 = 1;
pub const CB_SHOWDROPDOWN: UINT = 0x014F;
pub const WM_MOUSEHWHEEL: UINT = 0x020E;
pub const CB_GETDROPPEDSTATE: UINT = 0x0157;
pub const CBN_CLOSEUP: u16 = 8;
pub const CBN_SELENDOK: u16 = 9;
pub const CBN_SELENDCANCEL: u16 = 10;
pub const CBN_EDITCHANGE: u16 = 5;

pub const LBS_NOTIFY: u32 = 0x0001;
pub const LBS_SORT: u32 = 0x0002;
pub const LBS_MULTIPLESEL: u32 = 0x0008;
pub const LBS_NOINTEGRALHEIGHT: u32 = 0x0100;
pub const LBS_EXTENDEDSEL: u32 = 0x0800;
pub const LBS_OWNERDRAWFIXED: u32 = 0x0010;
pub const LBS_HASSTRINGS: u32 = 0x0040;
pub const LB_GETITEMDATA: UINT = 0x0199;
pub const LB_SETITEMDATA: UINT = 0x019A;
pub const LB_SETITEMHEIGHT: UINT = 0x01A0;
pub const LB_GETITEMHEIGHT: UINT = 0x01A1;
pub const LB_ADDSTRING: UINT = 0x0180;
pub const LB_INSERTSTRING: UINT = 0x0181;
pub const LB_DELETESTRING: UINT = 0x0182;
pub const LB_RESETCONTENT: UINT = 0x0184;
pub const LB_SETSEL: UINT = 0x0185;
pub const LB_SETCURSEL: UINT = 0x0186;
pub const LB_GETSEL: UINT = 0x0187;
pub const LB_GETCURSEL: UINT = 0x0188;
pub const LB_GETTEXT: UINT = 0x0189;
pub const LB_GETTEXTLEN: UINT = 0x018A;
pub const LB_GETCOUNT: UINT = 0x018B;
pub const LBN_SELCHANGE: u16 = 1;
pub const LBN_DBLCLK: u16 = 2;

pub const SBS_HORZ: u32 = 0x0000;
pub const SBS_VERT: u32 = 0x0001;
pub const SB_CTL: i32 = 2;
pub const SIF_RANGE: UINT = 0x1;
pub const SIF_PAGE: UINT = 0x2;
pub const SIF_POS: UINT = 0x4;
pub const SIF_TRACKPOS: UINT = 0x10;
pub const SIF_ALL: UINT = SIF_RANGE | SIF_PAGE | SIF_POS | SIF_TRACKPOS;
pub const SB_LINEUP: u16 = 0;
pub const SB_LINEDOWN: u16 = 1;
pub const SB_PAGEUP: u16 = 2;
pub const SB_PAGEDOWN: u16 = 3;
pub const SB_THUMBPOSITION: u16 = 4;
pub const SB_THUMBTRACK: u16 = 5;
pub const SB_TOP: u16 = 6;
pub const SB_BOTTOM: u16 = 7;

// ---- Menus -----------------------------------------------------------------
pub const MF_STRING: UINT = 0x0000;
pub const MF_GRAYED: UINT = 0x0001;
pub const MF_CHECKED: UINT = 0x0008;
pub const MF_POPUP: UINT = 0x0010;
pub const MF_BYCOMMAND: UINT = 0x0000;
pub const MF_BYPOSITION: UINT = 0x0400;
pub const MF_SEPARATOR: UINT = 0x0800;
pub const MF_ENABLED: UINT = 0x0000;
pub const MF_UNCHECKED: UINT = 0x0000;
pub const MIIM_BITMAP: UINT = 0x0080;
pub const MIIM_STRING: UINT = 0x0040;
pub const MIIM_STATE: UINT = 0x0001;
pub const MIIM_ID: UINT = 0x0002;
pub const MIIM_SUBMENU: UINT = 0x0004;
pub const MIIM_FTYPE: UINT = 0x0100;
pub const MFT_SEPARATOR: UINT = 0x0800;
pub const MFT_RADIOCHECK: UINT = 0x0200;
pub const MFS_GRAYED: UINT = 0x0003;
pub const MFS_CHECKED: UINT = 0x0008;
pub const MFS_DEFAULT: UINT = 0x1000;
pub const WM_INITMENUPOPUP: UINT = 0x0117;
pub const WM_MENUSELECT: UINT = 0x011F;
pub const SC_KEYMENU: usize = 0xF100;
pub const TPM_RIGHTBUTTON: UINT = 0x0002;
pub const TPM_RETURNCMD: UINT = 0x0100;

// ---- Diversos --------------------------------------------------------------
pub const MB_OK: UINT = 0x0;
pub const MB_OKCANCEL: UINT = 0x1;
pub const MB_YESNOCANCEL: UINT = 0x3;
pub const MB_YESNO: UINT = 0x4;
pub const MB_RETRYCANCEL: UINT = 0x5;
pub const MB_ICONERROR: UINT = 0x10;
pub const MB_ICONQUESTION: UINT = 0x20;
pub const MB_ICONWARNING: UINT = 0x30;
pub const MB_ICONINFORMATION: UINT = 0x40;
pub const MB_TASKMODAL: UINT = 0x2000;

pub const IDC_ARROW: usize = 32512;
pub const IDC_IBEAM: usize = 32513;
pub const IDC_WAIT: usize = 32514;
pub const IDC_CROSS: usize = 32515;
pub const IDC_SIZENWSE: usize = 32642;
pub const IDC_SIZENESW: usize = 32643;
pub const IDC_SIZEWE: usize = 32644;
pub const IDC_SIZENS: usize = 32645;
pub const IDC_SIZEALL: usize = 32646;
pub const IDC_NO: usize = 32648;
pub const IDC_HAND: usize = 32649;
pub const IDC_APPSTARTING: usize = 32650;
pub const IDC_HELP: usize = 32651;
pub const IDI_APPLICATION: usize = 32512;
pub const IDI_ERROR: usize = 32513;
pub const IDI_QUESTION: usize = 32514;
pub const IDI_WARNING: usize = 32515;
pub const IDI_INFORMATION: usize = 32516;
pub const IDI_SHIELD: usize = 32518;
pub const IMAGE_BITMAP: UINT = 0;
pub const IMAGE_ICON: UINT = 1;
pub const IMAGE_CURSOR: UINT = 2;
pub const LR_LOADFROMFILE: UINT = 0x0010;
pub const LR_DEFAULTSIZE: UINT = 0x0040;
pub const LR_CREATEDIBSECTION: UINT = 0x2000;

pub const SM_CXSCREEN: i32 = 0;
pub const SM_CYSCREEN: i32 = 1;
pub const SM_CXICON: i32 = 11;
pub const SM_CXSMICON: i32 = 49;
pub const SM_XVIRTUALSCREEN: i32 = 76;
pub const SM_YVIRTUALSCREEN: i32 = 77;
pub const SM_CXVIRTUALSCREEN: i32 = 78;
pub const SM_CYVIRTUALSCREEN: i32 = 79;
pub const SM_CMONITORS: i32 = 80;
pub const SM_DIGITIZER: i32 = 94;
pub const SM_MAXIMUMTOUCHES: i32 = 95;

pub const SPI_GETNONCLIENTMETRICS: UINT = 0x0029;
pub const SPI_GETHIGHCONTRAST: UINT = 0x0042;
pub const SPI_GETSCREENREADER: UINT = 0x0046;
pub const SPI_GETKEYBOARDCUES: UINT = 0x100A;
pub const SPI_GETCLIENTAREAANIMATION: UINT = 0x1042;
pub const HCF_HIGHCONTRASTON: DWORD = 0x1;

pub const MONITOR_DEFAULTTONEAREST: DWORD = 2;
pub const MONITORINFOF_PRIMARY: DWORD = 1;
pub const COLOR_WINDOW: i32 = 5;
pub const COLOR_WINDOWTEXT: i32 = 8;
pub const COLOR_HIGHLIGHT: i32 = 13;
pub const COLOR_HIGHLIGHTTEXT: i32 = 14;
pub const COLOR_BTNFACE: i32 = 15;
pub const COLOR_GRAYTEXT: i32 = 17;
pub const COLOR_BTNTEXT: i32 = 18;
pub const COLOR_HOTLIGHT: i32 = 26;

pub const VK_SHIFT: i32 = 0x10;
pub const VK_CONTROL: i32 = 0x11;
pub const VK_MENU: i32 = 0x12;
pub const VK_LWIN: i32 = 0x5B;
pub const VK_RWIN: i32 = 0x5C;
pub const CF_UNICODETEXT: UINT = 13;
pub const CF_HDROP: UINT = 15;
pub const CF_DIB: UINT = 8;
pub const CF_DIBV5: UINT = 17;
pub const CF_ENHMETAFILE: UINT = 14;
pub const WM_CLIPBOARDUPDATE: UINT = 0x031D;

pub const DT_LEFT: UINT = 0x0000;
pub const DT_CENTER: UINT = 0x0001;
pub const DT_RIGHT: UINT = 0x0002;
pub const DT_VCENTER: UINT = 0x0004;
pub const DT_WORDBREAK: UINT = 0x0010;
pub const DT_SINGLELINE: UINT = 0x0020;
pub const DT_CALCRECT: UINT = 0x0400;
pub const DT_NOPREFIX: UINT = 0x0800;
pub const DT_END_ELLIPSIS: UINT = 0x8000;
pub const DT_HIDEPREFIX: UINT = 0x0010_0000;

pub const RDW_INVALIDATE: UINT = 0x0001;
pub const RDW_ERASE: UINT = 0x0004;
pub const RDW_ALLCHILDREN: UINT = 0x0080;
pub const RDW_UPDATENOW: UINT = 0x0100;
pub const RDW_FRAME: UINT = 0x0400;
pub const DI_NORMAL: UINT = 0x0003;
pub const EVENT_OBJECT_NAMECHANGE: DWORD = 0x800C;
pub const OBJID_CLIENT: i32 = -4;
pub const GW_HWNDPREV: UINT = 3;
pub const GW_OWNER: UINT = 4;
pub const PM_REMOVE: UINT = 1;
pub const SC_CLOSE: usize = 0xF060;

#[repr(C)]
pub struct WNDCLASSEXW {
    pub cbSize: UINT,
    pub style: UINT,
    pub lpfnWndProc: Option<WNDPROC>,
    pub cbClsExtra: i32,
    pub cbWndExtra: i32,
    pub hInstance: HINSTANCE,
    pub hIcon: HICON,
    pub hCursor: HCURSOR,
    pub hbrBackground: HBRUSH,
    pub lpszMenuName: PCWSTR,
    pub lpszClassName: PCWSTR,
    pub hIconSm: HICON,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: UINT,
    pub wParam: WPARAM,
    pub lParam: LPARAM,
    pub time: DWORD,
    pub pt: POINT,
}

#[repr(C)]
pub struct PAINTSTRUCT {
    pub hdc: HDC,
    pub fErase: BOOL,
    pub rcPaint: RECT,
    pub fRestore: BOOL,
    pub fIncUpdate: BOOL,
    pub rgbReserved: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct MINMAXINFO {
    pub ptReserved: POINT,
    pub ptMaxSize: POINT,
    pub ptMaxPosition: POINT,
    pub ptMinTrackSize: POINT,
    pub ptMaxTrackSize: POINT,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MONITORINFOEXW {
    pub cbSize: DWORD,
    pub rcMonitor: RECT,
    pub rcWork: RECT,
    pub dwFlags: DWORD,
    pub szDevice: [u16; 32],
}

#[repr(C)]
pub struct TRACKMOUSEEVENT {
    pub cbSize: DWORD,
    pub dwFlags: DWORD,
    pub hwndTrack: HWND,
    pub dwHoverTime: DWORD,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NMHDR {
    pub hwndFrom: HWND,
    pub idFrom: usize,
    pub code: UINT,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct WINDOWPLACEMENT {
    pub length: UINT,
    pub flags: UINT,
    pub showCmd: UINT,
    pub ptMinPosition: POINT,
    pub ptMaxPosition: POINT,
    pub rcNormalPosition: RECT,
}

#[repr(C)]
pub struct MENUITEMINFOW {
    pub cbSize: UINT,
    pub fMask: UINT,
    pub fType: UINT,
    pub fState: UINT,
    pub wID: UINT,
    pub hSubMenu: HMENU,
    pub hbmpChecked: HBITMAP,
    pub hbmpUnchecked: HBITMAP,
    pub dwItemData: usize,
    pub dwTypeData: PWSTR,
    pub cch: UINT,
    pub hbmpItem: HBITMAP,
}

#[repr(C)]
pub struct NONCLIENTMETRICSW {
    pub cbSize: UINT,
    pub iBorderWidth: i32,
    pub iScrollWidth: i32,
    pub iScrollHeight: i32,
    pub iCaptionWidth: i32,
    pub iCaptionHeight: i32,
    pub lfCaptionFont: LOGFONTW,
    pub iSmCaptionWidth: i32,
    pub iSmCaptionHeight: i32,
    pub lfSmCaptionFont: LOGFONTW,
    pub iMenuWidth: i32,
    pub iMenuHeight: i32,
    pub lfMenuFont: LOGFONTW,
    pub lfStatusFont: LOGFONTW,
    pub lfMessageFont: LOGFONTW,
    pub iPaddedBorderWidth: i32,
}

#[repr(C)]
pub struct HIGHCONTRASTW {
    pub cbSize: UINT,
    pub dwFlags: DWORD,
    pub lpszDefaultScheme: PWSTR,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SCROLLINFO {
    pub cbSize: UINT,
    pub fMask: UINT,
    pub nMin: i32,
    pub nMax: i32,
    pub nPage: UINT,
    pub nPos: i32,
    pub nTrackPos: i32,
}

#[repr(C)]
pub struct DRAWITEMSTRUCT {
    pub CtlType: UINT,
    pub CtlID: UINT,
    pub itemID: UINT,
    pub itemAction: UINT,
    pub itemState: UINT,
    pub hwndItem: HWND,
    pub hDC: HDC,
    pub rcItem: RECT,
    pub itemData: usize,
}

#[repr(C)]
pub struct MEASUREITEMSTRUCT {
    pub CtlType: UINT,
    pub CtlID: UINT,
    pub itemID: UINT,
    pub itemWidth: UINT,
    pub itemHeight: UINT,
    pub itemData: usize,
}

#[repr(C)]
pub struct BUTTON_IMAGELIST {
    pub himl: HIMAGELIST,
    pub margin: RECT,
    pub uAlign: UINT,
}

#[repr(C)]
pub struct ICONINFO {
    pub fIcon: BOOL,
    pub xHotspot: DWORD,
    pub yHotspot: DWORD,
    pub hbmMask: HBITMAP,
    pub hbmColor: HBITMAP,
}

/// Callback for `EnumDisplayMonitors`.
pub type MONITORENUMPROC = unsafe extern "system" fn(HMONITOR, HDC, *mut RECT, LPARAM) -> BOOL;
/// Callback for `EnumChildWindows`.
pub type WNDENUMPROC = unsafe extern "system" fn(HWND, LPARAM) -> BOOL;

#[link(name = "user32")]
extern "system" {
    pub fn RegisterClassExW(class: *const WNDCLASSEXW) -> ATOM;
    pub fn CreateWindowExW(
        ex_style: DWORD,
        class_name: PCWSTR,
        window_name: PCWSTR,
        style: DWORD,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: HWND,
        menu: HMENU,
        instance: HINSTANCE,
        param: *mut c_void,
    ) -> HWND;
    pub fn DestroyWindow(hwnd: HWND) -> BOOL;
    pub fn DefWindowProcW(hwnd: HWND, msg: UINT, wp: WPARAM, lp: LPARAM) -> LRESULT;
    pub fn CallWindowProcW(previous: isize, hwnd: HWND, msg: UINT, wp: WPARAM, lp: LPARAM) -> LRESULT;
    pub fn ShowWindow(hwnd: HWND, cmd: i32) -> BOOL;
    pub fn UpdateWindow(hwnd: HWND) -> BOOL;
    pub fn GetMessageW(msg: *mut MSG, hwnd: HWND, min: UINT, max: UINT) -> BOOL;
    pub fn PeekMessageW(msg: *mut MSG, hwnd: HWND, min: UINT, max: UINT, remove: UINT) -> BOOL;
    pub fn TranslateMessage(msg: *const MSG) -> BOOL;
    pub fn DispatchMessageW(msg: *const MSG) -> LRESULT;
    pub fn PostQuitMessage(code: i32);
    pub fn PostMessageW(hwnd: HWND, msg: UINT, wp: WPARAM, lp: LPARAM) -> BOOL;
    pub fn SendMessageW(hwnd: HWND, msg: UINT, wp: WPARAM, lp: LPARAM) -> LRESULT;
    pub fn IsDialogMessageW(dialog: HWND, msg: *const MSG) -> BOOL;
    pub fn MessageBoxW(hwnd: HWND, text: PCWSTR, caption: PCWSTR, flags: UINT) -> i32;
    pub fn MessageBeep(kind: UINT) -> BOOL;
    pub fn SetWindowTextW(hwnd: HWND, text: PCWSTR) -> BOOL;
    pub fn GetWindowTextW(hwnd: HWND, buffer: PWSTR, max: i32) -> i32;
    pub fn GetWindowTextLengthW(hwnd: HWND) -> i32;
    pub fn EnableWindow(hwnd: HWND, enable: BOOL) -> BOOL;
    pub fn IsWindowEnabled(hwnd: HWND) -> BOOL;
    pub fn IsWindowVisible(hwnd: HWND) -> BOOL;
    pub fn IsWindow(hwnd: HWND) -> BOOL;
    pub fn IsIconic(hwnd: HWND) -> BOOL;
    pub fn IsZoomed(hwnd: HWND) -> BOOL;
    pub fn SetFocus(hwnd: HWND) -> HWND;
    pub fn GetFocus() -> HWND;
    pub fn IsChild(parent: HWND, child: HWND) -> BOOL;
    pub fn SetActiveWindow(hwnd: HWND) -> HWND;
    pub fn SetForegroundWindow(hwnd: HWND) -> BOOL;
    pub fn SetWindowPos(hwnd: HWND, after: HWND, x: i32, y: i32, cx: i32, cy: i32, flags: UINT) -> BOOL;
    pub fn GetWindowRect(hwnd: HWND, rect: *mut RECT) -> BOOL;
    pub fn GetClassNameW(hwnd: HWND, name: PWSTR, max: i32) -> i32;
    pub fn AddClipboardFormatListener(hwnd: HWND) -> BOOL;
    pub fn RemoveClipboardFormatListener(hwnd: HWND) -> BOOL;
    pub fn GetClipboardSequenceNumber() -> DWORD;
    pub fn GetClientRect(hwnd: HWND, rect: *mut RECT) -> BOOL;
    pub fn ScreenToClient(hwnd: HWND, point: *mut POINT) -> BOOL;
    pub fn GetCaretPos(point: *mut POINT) -> BOOL;
    pub fn AnimateWindow(hwnd: HWND, time: DWORD, flags: DWORD) -> BOOL;
    pub fn UpdateLayeredWindow(
        hwnd: HWND,
        hdc_dst: HDC,
        pt_dst: *const POINT,
        size: *const SIZE,
        hdc_src: HDC,
        pt_src: *const POINT,
        key: COLORREF,
        blend: *const super::gdi32::BLENDFUNCTION,
        flags: DWORD,
    ) -> BOOL;
    pub fn ClientToScreen(hwnd: HWND, point: *mut POINT) -> BOOL;
    pub fn MapWindowPoints(from: HWND, to: HWND, points: *mut POINT, count: UINT) -> i32;
    pub fn GetParent(hwnd: HWND) -> HWND;
    pub fn SetParent(child: HWND, parent: HWND) -> HWND;
    pub fn FindWindowW(class: PCWSTR, title: PCWSTR) -> HWND;
    pub fn GetAncestor(hwnd: HWND, flags: UINT) -> HWND;
    pub fn GetWindow(hwnd: HWND, cmd: UINT) -> HWND;
    // Em 32 bits a user32 não exporta as versões `Ptr`: lá elas são macros do
    // `windows.h` para `GetWindowLongW`/`SetWindowLongW`, que têm o mesmo tamanho de
    // ponteiro nesse alvo.
    #[cfg(target_pointer_width = "64")]
    pub fn GetWindowLongPtrW(hwnd: HWND, index: i32) -> isize;
    #[cfg(target_pointer_width = "32")]
    #[link_name = "GetWindowLongW"]
    pub fn GetWindowLongPtrW(hwnd: HWND, index: i32) -> isize;
    #[cfg(target_pointer_width = "64")]
    pub fn SetWindowLongPtrW(hwnd: HWND, index: i32, value: isize) -> isize;
    #[cfg(target_pointer_width = "32")]
    #[link_name = "SetWindowLongW"]
    pub fn SetWindowLongPtrW(hwnd: HWND, index: i32, value: isize) -> isize;
    pub fn InvalidateRect(hwnd: HWND, rect: *const RECT, erase: BOOL) -> BOOL;
    pub fn ValidateRect(hwnd: HWND, rect: *const RECT) -> BOOL;
    pub fn RedrawWindow(hwnd: HWND, rect: *const RECT, region: HRGN, flags: UINT) -> BOOL;
    pub fn GetNextDlgTabItem(dialog: HWND, control: HWND, previous: BOOL) -> HWND;
    pub fn PrintWindow(hwnd: HWND, hdc: HDC, flags: UINT) -> BOOL;
    pub fn BeginPaint(hwnd: HWND, ps: *mut PAINTSTRUCT) -> HDC;
    pub fn EndPaint(hwnd: HWND, ps: *const PAINTSTRUCT) -> BOOL;
    pub fn GetDC(hwnd: HWND) -> HDC;
    pub fn GetWindowDC(hwnd: HWND) -> HDC;
    pub fn GetComboBoxInfo(hwnd: HWND, info: *mut COMBOBOXINFO) -> BOOL;
    pub fn MoveWindow(hwnd: HWND, x: i32, y: i32, width: i32, height: i32, repaint: BOOL) -> BOOL;
    pub fn ReleaseDC(hwnd: HWND, hdc: HDC) -> i32;
    pub fn LoadCursorW(instance: HINSTANCE, name: PCWSTR) -> HCURSOR;
    pub fn LoadCursorFromFileW(path: PCWSTR) -> HCURSOR;
    pub fn LoadIconW(instance: HINSTANCE, name: PCWSTR) -> HICON;
    pub fn LoadImageW(instance: HINSTANCE, name: PCWSTR, kind: UINT, cx: i32, cy: i32, flags: UINT) -> HANDLE;
    pub fn SetCursor(cursor: HCURSOR) -> HCURSOR;
    pub fn DestroyIcon(icon: HICON) -> BOOL;
    pub fn DestroyCursor(cursor: HCURSOR) -> BOOL;
    pub fn CreateIconFromResourceEx(
        bits: *const u8,
        size: DWORD,
        icon: BOOL,
        version: DWORD,
        cx: i32,
        cy: i32,
        flags: UINT,
    ) -> HICON;
    pub fn CreateIconIndirect(info: *const ICONINFO) -> HICON;
    pub fn GetIconInfo(icon: HICON, info: *mut ICONINFO) -> BOOL;
    pub fn DrawIconEx(
        hdc: HDC,
        x: i32,
        y: i32,
        icon: HICON,
        cx: i32,
        cy: i32,
        step: UINT,
        brush: HBRUSH,
        flags: UINT,
    ) -> BOOL;
    pub fn GetSystemMetrics(index: i32) -> i32;
    pub fn SystemParametersInfoW(action: UINT, param: UINT, data: *mut c_void, flags: UINT) -> BOOL;
    pub fn MonitorFromWindow(hwnd: HWND, flags: DWORD) -> HMONITOR;
    pub fn MonitorFromPoint(point: POINT, flags: DWORD) -> HMONITOR;
    pub fn GetMonitorInfoW(monitor: HMONITOR, info: *mut MONITORINFOEXW) -> BOOL;
    pub fn EnumDisplayMonitors(hdc: HDC, clip: *const RECT, callback: MONITORENUMPROC, data: LPARAM) -> BOOL;
    pub fn EnumChildWindows(parent: HWND, callback: WNDENUMPROC, data: LPARAM) -> BOOL;
    pub fn SetLayeredWindowAttributes(hwnd: HWND, key: COLORREF, alpha: u8, flags: DWORD) -> BOOL;
    pub fn AdjustWindowRectEx(rect: *mut RECT, style: DWORD, menu: BOOL, ex_style: DWORD) -> BOOL;
    pub fn CreateMenu() -> HMENU;
    pub fn CreatePopupMenu() -> HMENU;
    pub fn DestroyMenu(menu: HMENU) -> BOOL;
    pub fn AppendMenuW(menu: HMENU, flags: UINT, id: usize, text: PCWSTR) -> BOOL;
    pub fn InsertMenuW(menu: HMENU, position: UINT, flags: UINT, id: usize, text: PCWSTR) -> BOOL;
    pub fn DeleteMenu(menu: HMENU, position: UINT, flags: UINT) -> BOOL;
    pub fn RemoveMenu(menu: HMENU, position: UINT, flags: UINT) -> BOOL;
    pub fn ModifyMenuW(menu: HMENU, position: UINT, flags: UINT, id: usize, text: PCWSTR) -> BOOL;
    pub fn GetMenuItemCount(menu: HMENU) -> i32;
    pub fn SetMenu(hwnd: HWND, menu: HMENU) -> BOOL;
    pub fn GetMenu(hwnd: HWND) -> HMENU;
    pub fn DrawMenuBar(hwnd: HWND) -> BOOL;
    pub fn TrackPopupMenu(
        menu: HMENU,
        flags: UINT,
        x: i32,
        y: i32,
        reserved: i32,
        hwnd: HWND,
        rect: *const RECT,
    ) -> BOOL;
    pub fn CheckMenuItem(menu: HMENU, id: UINT, flags: UINT) -> DWORD;
    pub fn EnableMenuItem(menu: HMENU, id: UINT, flags: UINT) -> BOOL;
    pub fn GetMenuState(menu: HMENU, id: UINT, flags: UINT) -> UINT;
    pub fn SetMenuItemInfoW(menu: HMENU, item: UINT, by_position: BOOL, info: *const MENUITEMINFOW) -> BOOL;
    pub fn GetMenuItemInfoW(menu: HMENU, item: UINT, by_position: BOOL, info: *mut MENUITEMINFOW) -> BOOL;
    pub fn CheckMenuRadioItem(menu: HMENU, first: UINT, last: UINT, check: UINT, flags: UINT) -> BOOL;
    pub fn GetMenuItemID(menu: HMENU, position: i32) -> UINT;
    pub fn WindowFromPoint(point: POINT) -> HWND;
    pub fn GetCapture() -> HWND;
    pub fn GetActiveWindow() -> HWND;
    pub fn GetSystemMenu(hwnd: HWND, revert: BOOL) -> HMENU;
    pub fn GetCursorPos(point: *mut POINT) -> BOOL;
    pub fn GetKeyState(vk: i32) -> i16;
    pub fn TrackMouseEvent(event: *mut TRACKMOUSEEVENT) -> BOOL;
    pub fn SetCapture(hwnd: HWND) -> HWND;
    pub fn SetWindowRgn(hwnd: HWND, region: HRGN, redraw: BOOL) -> i32;
    pub fn ReleaseCapture() -> BOOL;
    pub fn SetTimer(hwnd: HWND, id: usize, elapse: UINT, callback: *const c_void) -> usize;
    pub fn KillTimer(hwnd: HWND, id: usize) -> BOOL;
    pub fn OpenClipboard(owner: HWND) -> BOOL;
    pub fn CloseClipboard() -> BOOL;
    pub fn EmptyClipboard() -> BOOL;
    pub fn SetClipboardData(format: UINT, mem: HANDLE) -> HANDLE;
    pub fn GetClipboardData(format: UINT) -> HANDLE;
    pub fn IsClipboardFormatAvailable(format: UINT) -> BOOL;
    pub fn RegisterClipboardFormatW(name: PCWSTR) -> UINT;
    pub fn EnumClipboardFormats(format: UINT) -> UINT;
    pub fn GetClipboardFormatNameW(format: UINT, name: PWSTR, max: i32) -> i32;
    pub fn RegisterWindowMessageW(name: PCWSTR) -> UINT;
    pub fn FillRect(hdc: HDC, rect: *const RECT, brush: HBRUSH) -> i32;
    pub fn FrameRect(hdc: HDC, rect: *const RECT, brush: HBRUSH) -> i32;
    pub fn DrawTextW(hdc: HDC, text: PCWSTR, count: i32, rect: *mut RECT, format: UINT) -> i32;
    pub fn DrawFocusRect(hdc: HDC, rect: *const RECT) -> BOOL;
    pub fn GetSysColor(index: i32) -> DWORD;
    pub fn GetSysColorBrush(index: i32) -> HBRUSH;
    pub fn GetWindowPlacement(hwnd: HWND, placement: *mut WINDOWPLACEMENT) -> BOOL;
    pub fn SetWindowPlacement(hwnd: HWND, placement: *const WINDOWPLACEMENT) -> BOOL;
    pub fn GetDlgCtrlID(hwnd: HWND) -> i32;
    pub fn NotifyWinEvent(event: DWORD, hwnd: HWND, object: i32, child: i32);
    pub fn SetScrollInfo(hwnd: HWND, bar: i32, info: *const SCROLLINFO, redraw: BOOL) -> i32;
    pub fn GetScrollInfo(hwnd: HWND, bar: i32, info: *mut SCROLLINFO) -> BOOL;
    pub fn SetProcessDPIAware() -> BOOL;
    pub fn GetGuiResources(process: HANDLE, flags: DWORD) -> DWORD;
}

/// `WM_NCCALCSIZE` parameters when `WPARAM` is true.
#[repr(C)]
pub struct NCCALCSIZE_PARAMS {
    pub rgrc: [RECT; 3],
    pub lppos: *mut c_void,
}

/// Combo box parts (`GetComboBoxInfo`).
#[repr(C)]
pub struct COMBOBOXINFO {
    pub cbSize: DWORD,
    pub rcItem: RECT,
    pub rcButton: RECT,
    pub stateButton: DWORD,
    pub hwndCombo: HWND,
    pub hwndItem: HWND,
    pub hwndList: HWND,
}
