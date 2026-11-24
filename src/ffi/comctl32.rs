//! Declarations for `comctl32.dll` (Common Controls v6) and RichEdit.
//!
//! Resolves functions dynamically (see [`functions`]): v6 loads only after visual-style activation. Static import binds
//! v5.82, lacking APIs such as `TaskDialogIndirect`.

use super::types::*;
use std::ffi::c_void;
use std::sync::OnceLock;

pub const ICC_ALL_CLASSES: DWORD = 0xFFFF;
pub const ILC_MASK: UINT = 0x1;
pub const ILC_COLOR32: UINT = 0x20;

// Notificações comuns (valores negativos convertidos para UINT).
pub const NM_CLICK: i32 = -2;
pub const NM_DBLCLK: i32 = -3;
pub const NM_RETURN: i32 = -4;
pub const NM_RCLICK: i32 = -5;
pub const NM_CUSTOMDRAW: i32 = -12;
pub const CDDS_PREPAINT: DWORD = 0x1;
pub const CDDS_PREERASE: DWORD = 0x3;
pub const CDRF_DODEFAULT: LRESULT = 0x0;
pub const CDRF_SKIPDEFAULT: LRESULT = 0x4;
pub const CDIS_SELECTED: UINT = 0x1;
pub const CDIS_DISABLED: UINT = 0x4;
pub const CDIS_FOCUS: UINT = 0x10;
pub const CDIS_DEFAULT: UINT = 0x20;
pub const CDIS_HOT: UINT = 0x40;
pub const CDIS_SHOWKEYBOARDCUES: UINT = 0x200;
pub const CDIS_CHECKED: UINT = 0x8;
pub const CDIS_INDETERMINATE: UINT = 0x100;
pub const CDDS_ITEMPREPAINT: DWORD = 0x0001_0001;
pub const CDRF_NOTIFYITEMDRAW: LRESULT = 0x20;
pub const CDRF_NEWFONT: LRESULT = 0x2;
pub const CDRF_NOTIFYPOSTPAINT: LRESULT = 0x10;
pub const CDRF_NOTIFYSUBITEMDRAW: LRESULT = 0x20;
pub const CDDS_POSTPAINT: DWORD = 0x2;
pub const CDDS_ITEM: DWORD = 0x0001_0000;
pub const CDDS_SUBITEM: DWORD = 0x0002_0000;
pub const CDDS_ITEMPOSTPAINT: DWORD = 0x0001_0002;
pub const LVM_GETBKCOLOR: UINT = 0x1000;
pub const LVM_GETTEXTCOLOR: UINT = 0x1023;
pub const LVM_GETCOLUMNW: UINT = 0x105F;
pub const LVM_GETCOUNTPERPAGE: UINT = 0x1028;
pub const LVN_BEGINSCROLL: i32 = -180;
pub const LVN_ENDSCROLL: i32 = -181;
pub const HDN_ENDTRACKW: i32 = -327;
pub const HDN_TRACKW: i32 = -328;
pub const HDN_DIVIDERDBLCLICKW: i32 = -325;
pub const TBCDRF_NOEDGES: LRESULT = 0x0001_0000;
pub const TBCDRF_NOOFFSET: LRESULT = 0x0004_0000;
pub const TBCDRF_NOMARK: LRESULT = 0x0008_0000;
pub const TBCDRF_NOETCHEDEFFECT: LRESULT = 0x0010_0000;
pub const TBCDRF_NOBACKGROUND: LRESULT = 0x0040_0000;
pub const TBCDRF_USECDCOLORS: LRESULT = 0x0080_0000;
pub const TCM_GETITEMRECT: UINT = 0x130A;
pub const LVM_GETITEMRECT: UINT = 0x100E;
pub const LVM_HITTEST: UINT = 0x1012;
pub const LVM_SCROLL: UINT = 0x1014;
pub const LVM_GETTOPINDEX: UINT = 0x1027;
pub const LVN_BEGINDRAG: i32 = -109;
pub const TVN_BEGINDRAGW: i32 = -456;
pub const TVM_HITTEST: UINT = 0x1111;
pub const TVM_GETITEMRECT: UINT = 0x1104;
pub const TVGN_DROPHILITE: usize = 0x8;
pub const TVIS_EXPANDED: u32 = 0x20;
pub const TVIF_PARAM: u32 = 0x4;
pub const TVIF_CHILDREN: u32 = 0x40;
pub const LVIF_PARAM: u32 = 0x4;
pub const TCS_BOTTOM: u32 = 0x0002;
pub const TCS_VERTICAL: u32 = 0x0080;
pub const TCS_BUTTONS: u32 = 0x0100;
pub const SB_GETPARTS: UINT = 0x406;
pub const SB_GETRECT: UINT = 0x40A;
pub const SB_ISSIMPLE: UINT = 0x40E;
pub const SBT_OWNERDRAW: usize = 0x1000;
pub const HDM_GETITEMRECT: UINT = 0x1207;
pub const HDF_JUSTIFYMASK: i32 = 0x3;
pub const HDF_RIGHT: i32 = 0x1;
pub const HDF_CENTER: i32 = 0x2;
pub const MCM_SETCOLOR: UINT = 0x100A;
pub const MCSC_BACKGROUND: usize = 0;
pub const MCSC_TEXT: usize = 1;
pub const MCSC_TITLEBK: usize = 2;
pub const MCSC_TITLETEXT: usize = 3;
pub const MCSC_MONTHBK: usize = 4;
pub const MCSC_TRAILINGTEXT: usize = 5;
pub const DTM_SETMCCOLOR: UINT = 0x1006;
pub const DTM_GETMONTHCAL: UINT = 0x1008;
pub const DTN_DROPDOWN: i32 = -754;
pub const PBM_SETBARCOLOR: UINT = 0x409;
pub const PBM_SETBKCOLOR: UINT = 0x2001;
pub const CLR_DEFAULT: u32 = 0xFF00_0000;
pub const RB_SETBKCOLOR: UINT = 0x413;
pub const RB_SETTEXTCOLOR: UINT = 0x415;
pub const ILD_TRANSPARENT: UINT = 0x1;
pub const CCS_TOP: u32 = 0x1;
pub const CCS_NORESIZE: u32 = 0x4;
pub const CCS_NOPARENTALIGN: u32 = 0x8;
pub const CCS_NODIVIDER: u32 = 0x40;

// ListView
pub const LVM_SETBKCOLOR: UINT = 0x1001;
pub const LVM_SETIMAGELIST: UINT = 0x1003;
pub const LVM_GETITEMCOUNT: UINT = 0x1004;
pub const LVM_DELETEITEM: UINT = 0x1008;
pub const LVM_DELETEALLITEMS: UINT = 0x1009;
pub const LVM_GETNEXTITEM: UINT = 0x100C;
pub const LVM_ENSUREVISIBLE: UINT = 0x1013;
pub const LVM_REDRAWITEMS: UINT = 0x1015;
pub const LVM_DELETECOLUMN: UINT = 0x101C;
pub const LVM_GETCOLUMNWIDTH: UINT = 0x101D;
pub const LVM_SETCOLUMNWIDTH: UINT = 0x101E;
pub const LVM_GETHEADER: UINT = 0x101F;
pub const LVM_SETTEXTCOLOR: UINT = 0x1024;
pub const LVM_SETTEXTBKCOLOR: UINT = 0x1026;
pub const LVM_SETITEMSTATE: UINT = 0x102B;
pub const LVM_GETITEMSTATE: UINT = 0x102C;
pub const LVM_SETITEMCOUNT: UINT = 0x102F;
pub const LVM_GETSELECTEDCOUNT: UINT = 0x1032;
pub const LVM_SETEXTENDEDLISTVIEWSTYLE: UINT = 0x1036;
pub const LVM_GETSUBITEMRECT: UINT = 0x1038;
pub const LVM_SUBITEMHITTEST: UINT = 0x1039;
pub const LVM_GETITEMW: UINT = 0x104B;
pub const LVM_SETITEMW: UINT = 0x104C;
pub const LVM_INSERTITEMW: UINT = 0x104D;
pub const LVM_SORTITEMSEX: UINT = 0x1051;
pub const LVM_INSERTCOLUMNW: UINT = 0x1061;
pub const LVM_GETITEMTEXTW: UINT = 0x1073;
pub const LVM_SETITEMTEXTW: UINT = 0x1074;
pub const LVS_REPORT: u32 = 0x0001;
pub const LVS_SHAREIMAGELISTS: u32 = 0x0040;
pub const LVS_SINGLESEL: u32 = 0x0004;
pub const LVS_SHOWSELALWAYS: u32 = 0x0008;
pub const LVS_OWNERDATA: u32 = 0x1000;
pub const LVS_EX_GRIDLINES: u32 = 0x1;
pub const LVS_EX_CHECKBOXES: u32 = 0x4;
pub const LVS_EX_HEADERDRAGDROP: u32 = 0x10;
pub const LVS_EX_FULLROWSELECT: u32 = 0x20;
pub const LVS_EX_DOUBLEBUFFER: u32 = 0x10000;
pub const LVCF_FMT: u32 = 0x1;
pub const LVCF_WIDTH: u32 = 0x2;
pub const LVCF_TEXT: u32 = 0x4;
pub const LVCF_SUBITEM: u32 = 0x8;
pub const LVCFMT_LEFT: i32 = 0;
pub const LVCFMT_RIGHT: i32 = 1;
pub const LVCFMT_CENTER: i32 = 2;
pub const LVIF_TEXT: u32 = 0x1;
pub const LVIF_IMAGE: u32 = 0x2;
pub const LVIF_STATE: u32 = 0x8;
pub const LVIS_FOCUSED: u32 = 0x1;
pub const LVIS_SELECTED: u32 = 0x2;
pub const LVIS_STATEIMAGEMASK: u32 = 0xF000;
pub const LVNI_SELECTED: usize = 0x2;
pub const LVNI_FOCUSED: usize = 0x1;
pub const LVIR_BOUNDS: i32 = 0;
pub const LVIR_LABEL: i32 = 2;
pub const LVSIL_SMALL: usize = 1;
pub const LVN_ITEMCHANGED: i32 = -101;
pub const LVN_COLUMNCLICK: i32 = -108;
pub const LVN_ITEMACTIVATE: i32 = -114;
pub const LVN_GETDISPINFOW: i32 = -177;

// TreeView
pub const TVM_DELETEITEM: UINT = 0x1101;
pub const TVM_EXPAND: UINT = 0x1102;
pub const TVM_GETCOUNT: UINT = 0x1105;
pub const TVM_SETIMAGELIST: UINT = 0x1109;
pub const TVM_GETIMAGELIST: UINT = 0x1108;
pub const TVM_GETITEMSTATE: UINT = 0x1127;
pub const TVM_GETNEXTITEM: UINT = 0x110A;
pub const TVM_SELECTITEM: UINT = 0x110B;
pub const TVM_ENSUREVISIBLE: UINT = 0x1114;
pub const TVM_SETBKCOLOR: UINT = 0x111D;
pub const TVM_SETTEXTCOLOR: UINT = 0x111E;
pub const TVM_INSERTITEMW: UINT = 0x1132;
pub const TVM_GETITEMW: UINT = 0x113E;
pub const TVM_SETITEMW: UINT = 0x113F;
pub const TVS_HASBUTTONS: u32 = 0x1;
pub const TVS_HASLINES: u32 = 0x2;
pub const TVS_LINESATROOT: u32 = 0x4;
pub const TVS_SHOWSELALWAYS: u32 = 0x20;
pub const TVS_CHECKBOXES: u32 = 0x100;
pub const TVIF_TEXT: u32 = 0x1;
pub const TVIF_IMAGE: u32 = 0x2;
pub const TVIF_STATE: u32 = 0x8;
pub const TVIF_HANDLE: u32 = 0x10;
pub const TVIF_SELECTEDIMAGE: u32 = 0x20;
pub const TVIS_STATEIMAGEMASK: u32 = 0xF000;
pub const TVI_ROOT: HTREEITEM = -0x10000isize as HTREEITEM;
pub const TVI_LAST: HTREEITEM = -0xFFFEisize as HTREEITEM;
pub const TVGN_ROOT: usize = 0;
pub const TVGN_NEXT: usize = 1;
pub const TVGN_PREVIOUS: usize = 2;
pub const TVI_FIRST: HTREEITEM = -0xFFFFisize as HTREEITEM;
pub const TVGN_PARENT: usize = 3;
pub const TVGN_CHILD: usize = 4;
pub const TVGN_CARET: usize = 9;
pub const TVE_COLLAPSE: usize = 1;
pub const TVE_EXPAND: usize = 2;
pub const TVSIL_NORMAL: usize = 0;
pub const TVN_SELCHANGEDW: i32 = -451;
pub const TVN_ITEMEXPANDEDW: i32 = -455;

// TabControl
pub const TCM_SETIMAGELIST: UINT = 0x1303;
pub const TCM_GETITEMCOUNT: UINT = 0x1304;
pub const TCM_DELETEITEM: UINT = 0x1308;
pub const TCM_GETCURSEL: UINT = 0x130B;
pub const TCM_SETCURSEL: UINT = 0x130C;
pub const TCM_ADJUSTRECT: UINT = 0x1328;
pub const TCM_GETITEMW: UINT = 0x133C;
pub const TCM_SETITEMW: UINT = 0x133D;
pub const TCM_INSERTITEMW: UINT = 0x133E;
pub const TCIF_TEXT: u32 = 0x1;
pub const TCIF_IMAGE: u32 = 0x2;
pub const TCN_SELCHANGE: i32 = -551;

// ProgressBar / TrackBar / UpDown
pub const PBM_SETPOS: UINT = 0x402;
pub const PBM_SETSTEP: UINT = 0x404;
pub const PBM_STEPIT: UINT = 0x405;
pub const PBM_SETRANGE32: UINT = 0x406;
pub const PBM_GETPOS: UINT = 0x408;
pub const PBM_SETMARQUEE: UINT = 0x40A;
pub const PBM_SETSTATE: UINT = 0x410;
pub const PBS_SMOOTH: u32 = 0x1;
pub const PBS_VERTICAL: u32 = 0x4;
pub const PBS_MARQUEE: u32 = 0x8;
pub const TBM_GETPOS: UINT = 0x400;
pub const TBM_SETPOS: UINT = 0x405;
pub const TBM_SETRANGEMIN: UINT = 0x407;
pub const TBM_SETRANGEMAX: UINT = 0x408;
pub const TBM_SETTICFREQ: UINT = 0x414;
pub const TBM_SETPAGESIZE: UINT = 0x415;
pub const TBS_AUTOTICKS: u32 = 0x1;
pub const TBS_VERT: u32 = 0x2;
pub const UDM_SETBUDDY: UINT = 0x469;
pub const UDM_SETRANGE32: UINT = 0x46F;
pub const UDM_SETPOS32: UINT = 0x471;
pub const UDM_GETPOS32: UINT = 0x472;
pub const UDS_SETBUDDYINT: u32 = 0x2;
pub const UDS_ALIGNRIGHT: u32 = 0x4;
pub const UDS_ARROWKEYS: u32 = 0x20;
pub const UDS_NOTHOUSANDS: u32 = 0x80;
pub const UDS_HORZ: u32 = 0x40;
pub const UDN_DELTAPOS: i32 = -722;

// DateTimePicker / MonthCalendar
pub const DTM_GETSYSTEMTIME: UINT = 0x1001;
pub const DTM_SETSYSTEMTIME: UINT = 0x1002;
pub const DTM_SETFORMATW: UINT = 0x1032;
pub const DTS_UPDOWN: u32 = 0x1;
pub const DTS_SHOWNONE: u32 = 0x2;
pub const DTS_LONGDATEFORMAT: u32 = 0x4;
pub const DTS_TIMEFORMAT: u32 = 0x9;
pub const DTN_DATETIMECHANGE: i32 = -759;
pub const GDT_VALID: usize = 0;
pub const GDT_NONE: usize = 1;
pub const MCM_GETCURSEL: UINT = 0x1001;
pub const MCM_SETCURSEL: UINT = 0x1002;
pub const MCM_GETMINREQRECT: UINT = 0x1009;
pub const MCN_SELCHANGE: i32 = -749;

// StatusBar / ToolBar / Tooltip
pub const SB_SETPARTS: UINT = 0x404;
pub const SB_SETTEXTW: UINT = 0x40B;
pub const SB_GETTEXTLENGTHW: UINT = 0x40C;
pub const SB_GETTEXTW: UINT = 0x40D;
pub const SBARS_SIZEGRIP: u32 = 0x100;
pub const TB_ENABLEBUTTON: UINT = 0x401;
pub const TB_CHECKBUTTON: UINT = 0x402;
pub const TB_ISBUTTONENABLED: UINT = 0x409;
pub const TB_ISBUTTONCHECKED: UINT = 0x40A;
pub const TB_BUTTONCOUNT: UINT = 0x418;
pub const TB_BUTTONSTRUCTSIZE: UINT = 0x41E;
pub const TB_AUTOSIZE: UINT = 0x421;
pub const TB_SETIMAGELIST: UINT = 0x430;
pub const TB_GETBUTTONSIZE: UINT = 0x43A;
pub const TB_ADDBUTTONSW: UINT = 0x444;
pub const TB_SETEXTENDEDSTYLE: UINT = 0x454;
pub const TB_GETMAXSIZE: UINT = 0x453;
pub const TBSTYLE_TOOLTIPS: u32 = 0x100;
pub const TBSTYLE_FLAT: u32 = 0x800;
pub const TBSTYLE_LIST: u32 = 0x1000;
pub const TBSTATE_ENABLED: u8 = 0x4;
pub const BTNS_BUTTON: u8 = 0x0;
pub const BTNS_SEP: u8 = 0x1;
pub const BTNS_CHECK: u8 = 0x2;
pub const BTNS_AUTOSIZE: u8 = 0x10;
pub const BTNS_SHOWTEXT: u8 = 0x40;
pub const I_IMAGENONE: i32 = -2;
pub const TBSTYLE_EX_MIXEDBUTTONS: usize = 0x8;
pub const TTM_ACTIVATE: UINT = 0x401;
pub const TTM_SETMAXTIPWIDTH: UINT = 0x418;
pub const TTM_ADDTOOLW: UINT = 0x432;
pub const TTM_DELTOOLW: UINT = 0x433;
pub const TTM_UPDATETIPTEXTW: UINT = 0x439;
pub const TTS_ALWAYSTIP: u32 = 0x1;
pub const TTS_NOPREFIX: u32 = 0x2;
pub const TTS_BALLOON: u32 = 0x40;
pub const TTF_IDISHWND: UINT = 0x1;
pub const TTF_SUBCLASS: UINT = 0x10;

// Header / Rebar / Pager / HotKey / IPAddress / Animate / SysLink
pub const HDM_GETITEMCOUNT: UINT = 0x1200;
pub const HDM_DELETEITEM: UINT = 0x1202;
pub const HDM_INSERTITEMW: UINT = 0x120A;
pub const HDM_GETITEMW: UINT = 0x120B;
pub const HDM_SETITEMW: UINT = 0x120C;
pub const HDI_WIDTH: u32 = 0x1;
pub const HDI_TEXT: u32 = 0x2;
pub const HDI_FORMAT: u32 = 0x4;
pub const HDF_STRING: i32 = 0x4000;
pub const HDF_SORTDOWN: i32 = 0x200;
pub const HDF_SORTUP: i32 = 0x400;
pub const HDS_BUTTONS: u32 = 0x2;
pub const HDN_ITEMCHANGEDW: i32 = -321;
pub const HDN_ITEMCLICKW: i32 = -322;
pub const RB_DELETEBAND: UINT = 0x402;
pub const RB_INSERTBANDW: UINT = 0x40A;
pub const RB_GETBANDCOUNT: UINT = 0x40C;
pub const RB_GETBARHEIGHT: UINT = 0x41B;
pub const RBBIM_STYLE: u32 = 0x1;
pub const RBBIM_TEXT: u32 = 0x4;
pub const RBBIM_CHILD: u32 = 0x10;
pub const RBBIM_CHILDSIZE: u32 = 0x20;
pub const RBBIM_SIZE: u32 = 0x40;
pub const RBBS_BREAK: u32 = 0x1;
pub const RBBS_CHILDEDGE: u32 = 0x4;
pub const RBBS_GRIPPERALWAYS: u32 = 0x80;
pub const RBS_VARHEIGHT: u32 = 0x200;
pub const RBS_BANDBORDERS: u32 = 0x400;
pub const PGM_SETCHILD: UINT = 0x1401;
pub const PGM_RECALCSIZE: UINT = 0x1402;
pub const PGM_SETPOS: UINT = 0x1408;
pub const PGM_GETPOS: UINT = 0x1409;
pub const PGM_SETBUTTONSIZE: UINT = 0x140A;
pub const PGS_HORZ: u32 = 0x1;
pub const PGN_CALCSIZE: i32 = -902;
pub const PGF_CALCWIDTH: DWORD = 1;
pub const PGF_CALCHEIGHT: DWORD = 2;
pub const HKM_SETHOTKEY: UINT = 0x401;
pub const HKM_GETHOTKEY: UINT = 0x402;
pub const HOTKEYF_SHIFT: u8 = 0x1;
pub const HOTKEYF_CONTROL: u8 = 0x2;
pub const HOTKEYF_ALT: u8 = 0x4;
pub const IPM_CLEARADDRESS: UINT = 0x464;
pub const IPM_SETADDRESS: UINT = 0x465;
pub const IPM_GETADDRESS: UINT = 0x466;
pub const IPN_FIELDCHANGED: i32 = -860;
pub const ACM_PLAY: UINT = 0x465;
pub const ACM_STOP: UINT = 0x466;
pub const ACM_OPENW: UINT = 0x467;
pub const ACS_CENTER: u32 = 0x1;
pub const ACS_TRANSPARENT: u32 = 0x2;
pub const ACS_AUTOPLAY: u32 = 0x4;

// RichEdit
pub const EM_EXLIMITTEXT: UINT = 0x435;
pub const EM_GETCHARFORMAT: UINT = 0x43A;
pub const EM_SETBKGNDCOLOR: UINT = 0x443;
pub const EM_SETCHARFORMAT: UINT = 0x444;
pub const EM_SETEVENTMASK: UINT = 0x445;
pub const EM_SETPARAFORMAT: UINT = 0x447;
pub const EM_STREAMIN: UINT = 0x449;
pub const EM_STREAMOUT: UINT = 0x44A;
pub const ENM_CHANGE: LPARAM = 0x1;
pub const ENM_SELCHANGE: LPARAM = 0x80000;
pub const EN_SELCHANGE: i32 = 0x702;
pub const SCF_SELECTION: WPARAM = 0x1;
pub const SCF_ALL: WPARAM = 0x4;
pub const CFM_BOLD: DWORD = 0x1;
pub const CFM_ITALIC: DWORD = 0x2;
pub const CFM_UNDERLINE: DWORD = 0x4;
pub const CFM_STRIKEOUT: DWORD = 0x8;
pub const CFM_BACKCOLOR: DWORD = 0x0400_0000;
pub const CFM_FACE: DWORD = 0x2000_0000;
pub const CFM_COLOR: DWORD = 0x4000_0000;
pub const CFM_SIZE: DWORD = 0x8000_0000;
pub const CFE_AUTOCOLOR: DWORD = 0x4000_0000;
pub const SF_TEXT: WPARAM = 0x1;
pub const SF_RTF: WPARAM = 0x2;
pub const PFM_ALIGNMENT: DWORD = 0x8;
pub const PFM_OFFSET: DWORD = 0x4;
pub const PFM_STARTINDENT: DWORD = 0x1;
pub const PFM_NUMBERING: DWORD = 0x20;
pub const PFM_NUMBERINGSTYLE: DWORD = 0x2000;
pub const PFM_NUMBERINGTAB: DWORD = 0x4000;
pub const PFM_NUMBERINGSTART: DWORD = 0x8000;
pub const PFN_BULLET: u16 = 1;
pub const PFN_ARABIC: u16 = 2;
pub const PFN_LCLETTER: u16 = 3;
pub const PFN_UCLETTER: u16 = 4;
pub const PFN_LCROMAN: u16 = 5;
pub const PFN_UCROMAN: u16 = 6;
pub const PFNS_PERIOD: u16 = 0x200;
pub const EM_GETPARAFORMAT: UINT = 0x43D;
pub const SFF_SELECTION: WPARAM = 0x8000;

// TaskDialog
pub const TDCBF_OK_BUTTON: DWORD = 0x1;
pub const TDCBF_YES_BUTTON: DWORD = 0x2;
pub const TDCBF_NO_BUTTON: DWORD = 0x4;
pub const TDCBF_CANCEL_BUTTON: DWORD = 0x8;
pub const TDCBF_RETRY_BUTTON: DWORD = 0x10;
pub const TDCBF_CLOSE_BUTTON: DWORD = 0x20;
pub const TDF_ALLOW_DIALOG_CANCELLATION: DWORD = 0x8;
pub const TDF_USE_COMMAND_LINKS: DWORD = 0x10;
pub const TDF_VERIFICATION_FLAG_CHECKED: DWORD = 0x100;
pub const TDF_POSITION_RELATIVE_TO_WINDOW: DWORD = 0x1000;
pub const TDF_SIZE_TO_CONTENT: DWORD = 0x0100_0000;
pub const TDF_USE_HICON_MAIN: DWORD = 0x2;
pub const SB_SETICON: UINT = 0x40F;
pub const SB_GETICON: UINT = 0x414;
pub const TCM_GETIMAGELIST: UINT = 0x1302;

#[repr(C)]
pub struct INITCOMMONCONTROLSEX {
    pub dwSize: DWORD,
    pub dwICC: DWORD,
}

#[repr(C)]
pub struct LVCOLUMNW {
    pub mask: u32,
    pub fmt: i32,
    pub cx: i32,
    pub pszText: PWSTR,
    pub cchTextMax: i32,
    pub iSubItem: i32,
    pub iImage: i32,
    pub iOrder: i32,
    pub cxMin: i32,
    pub cxDefault: i32,
    pub cxIdeal: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LVITEMW {
    pub mask: u32,
    pub iItem: i32,
    pub iSubItem: i32,
    pub state: u32,
    pub stateMask: u32,
    pub pszText: PWSTR,
    pub cchTextMax: i32,
    pub iImage: i32,
    pub lParam: LPARAM,
    pub iIndent: i32,
    pub iGroupId: i32,
    pub cColumns: u32,
    pub puColumns: *mut u32,
    pub piColFmt: *mut i32,
    pub iGroup: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NMLISTVIEW {
    pub hdr: super::user32::NMHDR,
    pub iItem: i32,
    pub iSubItem: i32,
    pub uNewState: u32,
    pub uOldState: u32,
    pub uChanged: u32,
    pub ptAction: POINT,
    pub lParam: LPARAM,
}

#[repr(C)]
pub struct NMLVDISPINFOW {
    pub hdr: super::user32::NMHDR,
    pub item: LVITEMW,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct LVHITTESTINFO {
    pub pt: POINT,
    pub flags: u32,
    pub iItem: i32,
    pub iSubItem: i32,
    pub iGroup: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TVITEMW {
    pub mask: u32,
    pub hItem: HTREEITEM,
    pub state: u32,
    pub stateMask: u32,
    pub pszText: PWSTR,
    pub cchTextMax: i32,
    pub iImage: i32,
    pub iSelectedImage: i32,
    pub cChildren: i32,
    pub lParam: LPARAM,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TVITEMEXW {
    pub base: TVITEMW,
    pub iIntegral: i32,
    pub uStateEx: u32,
    pub hwnd: HWND,
    pub iExpandedImage: i32,
    pub iReserved: i32,
}

#[repr(C)]
pub struct TVINSERTSTRUCTW {
    pub hParent: HTREEITEM,
    pub hInsertAfter: HTREEITEM,
    pub item: TVITEMEXW,
}

#[repr(C)]
pub struct NMTREEVIEWW {
    pub hdr: super::user32::NMHDR,
    pub action: u32,
    pub itemOld: TVITEMW,
    pub itemNew: TVITEMW,
    pub ptDrag: POINT,
}

#[repr(C)]
pub struct TCITEMW {
    pub mask: u32,
    pub dwState: u32,
    pub dwStateMask: u32,
    pub pszText: PWSTR,
    pub cchTextMax: i32,
    pub iImage: i32,
    pub lParam: LPARAM,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TBBUTTON {
    pub iBitmap: i32,
    pub idCommand: i32,
    pub fsState: u8,
    pub fsStyle: u8,
    #[cfg(target_pointer_width = "64")]
    pub bReserved: [u8; 6],
    #[cfg(target_pointer_width = "32")]
    pub bReserved: [u8; 2],
    pub dwData: usize,
    pub iString: isize,
}

#[repr(C)]
pub struct TTTOOLINFOW {
    pub cbSize: UINT,
    pub uFlags: UINT,
    pub hwnd: HWND,
    pub uId: usize,
    pub rect: RECT,
    pub hinst: HINSTANCE,
    pub lpszText: PWSTR,
    pub lParam: LPARAM,
    pub lpReserved: *mut c_void,
}

#[repr(C)]
pub struct NMCUSTOMDRAW {
    pub hdr: super::user32::NMHDR,
    pub dwDrawStage: DWORD,
    pub hdc: HDC,
    pub rc: RECT,
    pub dwItemSpec: usize,
    pub uItemState: UINT,
    pub lItemlParam: LPARAM,
}

/// `NMLVCUSTOMDRAW` (ListView custom drawing).
#[repr(C)]
pub struct NMLVCUSTOMDRAW {
    pub nmcd: NMCUSTOMDRAW,
    pub clrText: u32,
    pub clrTextBk: u32,
    pub iSubItem: i32,
    pub dwItemType: DWORD,
    pub clrFace: u32,
    pub iIconEffect: i32,
    pub iIconPhase: i32,
    pub iPartId: i32,
    pub iStateId: i32,
    pub rcText: RECT,
    pub uAlign: UINT,
}

#[repr(C)]
pub struct TVHITTESTINFO {
    pub pt: POINT,
    pub flags: u32,
    pub hItem: HTREEITEM,
}

/// `NMTBCUSTOMDRAW` (toolbar custom drawing).
#[repr(C)]
pub struct NMTBCUSTOMDRAW {
    pub nmcd: NMCUSTOMDRAW,
    pub hbrMonoDither: HBRUSH,
    pub hbrLines: HBRUSH,
    pub hpenLines: HPEN,
    pub clrText: u32,
    pub clrMark: u32,
    pub clrTextHighlight: u32,
    pub clrBtnFace: u32,
    pub clrBtnHighlight: u32,
    pub clrHighlightHotTrack: u32,
    pub rcText: RECT,
    pub nStringBkMode: i32,
    pub nHLStringBkMode: i32,
    pub iListGap: i32,
}

#[repr(C)]
pub struct HDITEMW {
    pub mask: u32,
    pub cxy: i32,
    pub pszText: PWSTR,
    pub hbm: HBITMAP,
    pub cchTextMax: i32,
    pub fmt: i32,
    pub lParam: LPARAM,
    pub iImage: i32,
    pub iOrder: i32,
    pub r#type: u32,
    pub pvFilter: *mut c_void,
    pub state: u32,
}

#[repr(C)]
pub struct NMHEADERW {
    pub hdr: super::user32::NMHDR,
    pub iItem: i32,
    pub iButton: i32,
    pub pitem: *mut HDITEMW,
}

#[repr(C)]
pub struct REBARBANDINFOW {
    pub cbSize: u32,
    pub fMask: u32,
    pub fStyle: u32,
    pub clrFore: COLORREF,
    pub clrBack: COLORREF,
    pub lpText: PWSTR,
    pub cch: u32,
    pub iImage: i32,
    pub hwndChild: HWND,
    pub cxMinChild: u32,
    pub cyMinChild: u32,
    pub cx: u32,
    pub hbmBack: HBITMAP,
    pub wID: u32,
    pub cyChild: u32,
    pub cyMaxChild: u32,
    pub cyIntegral: u32,
    pub cxIdeal: u32,
    pub lParam: LPARAM,
    pub cxHeader: u32,
    pub rcChevronLocation: RECT,
    pub uChevronState: u32,
}

#[repr(C)]
pub struct NMUPDOWN {
    pub hdr: super::user32::NMHDR,
    pub iPos: i32,
    pub iDelta: i32,
}

#[repr(C)]
pub struct NMPGCALCSIZE {
    pub hdr: super::user32::NMHDR,
    pub dwFlag: DWORD,
    pub iWidth: i32,
    pub iHeight: i32,
}

#[repr(C)]
pub struct LITEM {
    pub mask: u32,
    pub iLink: i32,
    pub state: u32,
    pub stateMask: u32,
    pub szID: [u16; 48],
    pub szUrl: [u16; 2084],
}

#[repr(C)]
pub struct NMLINK {
    pub hdr: super::user32::NMHDR,
    pub item: LITEM,
}

/// `CHARFORMAT2W` — `richedit.h` uses 4-byte packing.
#[repr(C, packed(4))]
#[derive(Clone, Copy)]
pub struct CHARFORMAT2W {
    pub cbSize: u32,
    pub dwMask: DWORD,
    pub dwEffects: DWORD,
    pub yHeight: i32,
    pub yOffset: i32,
    pub crTextColor: COLORREF,
    pub bCharSet: u8,
    pub bPitchAndFamily: u8,
    pub szFaceName: [u16; 32],
    pub wWeight: u16,
    pub sSpacing: i16,
    pub crBackColor: COLORREF,
    pub lcid: u32,
    pub dwReserved: DWORD,
    pub sStyle: i16,
    pub wKerning: u16,
    pub bUnderlineType: u8,
    pub bAnimation: u8,
    pub bRevAuthor: u8,
    pub bUnderlineColor: u8,
}

/// `PARAFORMAT` (4-byte packing).
#[repr(C, packed(4))]
#[derive(Clone, Copy)]
pub struct PARAFORMAT {
    pub cbSize: u32,
    pub dwMask: DWORD,
    pub wNumbering: u16,
    pub wReserved: u16,
    pub dxStartIndent: i32,
    pub dxRightIndent: i32,
    pub dxOffset: i32,
    pub wAlignment: u16,
    pub cTabCount: i16,
    pub rgxTabs: [i32; 32],
}

/// `PARAFORMAT2` (4-byte packing): `PARAFORMAT` plus spacing, numbering and borders.
#[repr(C, packed(4))]
#[derive(Clone, Copy)]
pub struct PARAFORMAT2 {
    pub base: PARAFORMAT,
    pub dySpaceBefore: i32,
    pub dySpaceAfter: i32,
    pub dyLineSpacing: i32,
    pub sStyle: i16,
    pub bLineSpacingRule: u8,
    pub bOutlineLevel: u8,
    pub wShadingWeight: u16,
    pub wShadingStyle: u16,
    pub wNumberingStart: u16,
    pub wNumberingStyle: u16,
    pub wNumberingTab: u16,
    pub wBorderSpace: u16,
    pub wBorderWidth: u16,
    pub wBorders: u16,
}

/// Callback for `EM_STREAMIN`/`EM_STREAMOUT`.
pub type EDITSTREAMCALLBACK = unsafe extern "system" fn(usize, *mut u8, i32, *mut i32) -> DWORD;

/// `EDITSTREAM` (4-byte packing).
#[repr(C, packed(4))]
pub struct EDITSTREAM {
    pub dwCookie: usize,
    pub dwError: DWORD,
    pub pfnCallback: Option<EDITSTREAMCALLBACK>,
}

#[repr(C, packed(1))]
#[derive(Clone, Copy)]
pub struct TASKDIALOG_BUTTON {
    pub nButtonID: i32,
    pub pszButtonText: PCWSTR,
}

#[repr(C, packed(1))]
pub struct TASKDIALOGCONFIG {
    pub cbSize: UINT,
    pub hwndParent: HWND,
    pub hInstance: HINSTANCE,
    pub dwFlags: DWORD,
    pub dwCommonButtons: DWORD,
    pub pszWindowTitle: PCWSTR,
    pub pszMainIcon: PCWSTR,
    pub pszMainInstruction: PCWSTR,
    pub pszContent: PCWSTR,
    pub cButtons: UINT,
    pub pButtons: *const TASKDIALOG_BUTTON,
    pub nDefaultButton: i32,
    pub cRadioButtons: UINT,
    pub pRadioButtons: *const TASKDIALOG_BUTTON,
    pub nDefaultRadioButton: i32,
    pub pszVerificationText: PCWSTR,
    pub pszExpandedInformation: PCWSTR,
    pub pszExpandedControlText: PCWSTR,
    pub pszCollapsedControlText: PCWSTR,
    pub pszFooterIcon: PCWSTR,
    pub pszFooter: PCWSTR,
    pub pfCallback: *const c_void,
    pub lpCallbackData: isize,
    pub cxWidth: UINT,
}

type InitCommonControlsExFn = unsafe extern "system" fn(*const INITCOMMONCONTROLSEX) -> BOOL;
type ImageListCreateFn = unsafe extern "system" fn(i32, i32, UINT, i32, i32) -> HIMAGELIST;
type ImageListDestroyFn = unsafe extern "system" fn(HIMAGELIST) -> BOOL;
type ImageListReplaceIconFn = unsafe extern "system" fn(HIMAGELIST, i32, HICON) -> i32;
type ImageListAddFn = unsafe extern "system" fn(HIMAGELIST, HBITMAP, HBITMAP) -> i32;
type ImageListGetImageCountFn = unsafe extern "system" fn(HIMAGELIST) -> i32;
type ImageListGetIconSizeFn = unsafe extern "system" fn(HIMAGELIST, *mut i32, *mut i32) -> BOOL;
type ImageListDrawFn = unsafe extern "system" fn(HIMAGELIST, i32, HDC, i32, i32, UINT) -> BOOL;
type TaskDialogIndirectFn =
    unsafe extern "system" fn(*const TASKDIALOGCONFIG, *mut i32, *mut i32, *mut BOOL) -> HRESULT;

/// Runtime-resolved `comctl32.dll` function table.
pub struct ComCtlFunctions {
    pub InitCommonControlsEx: InitCommonControlsExFn,
    pub ImageList_Create: ImageListCreateFn,
    pub ImageList_Destroy: ImageListDestroyFn,
    pub ImageList_ReplaceIcon: ImageListReplaceIconFn,
    pub ImageList_Add: ImageListAddFn,
    pub ImageList_GetImageCount: ImageListGetImageCountFn,
    pub ImageList_GetIconSize: ImageListGetIconSizeFn,
    pub ImageList_Draw: ImageListDrawFn,
    /// Available only in DLL version 6.
    pub TaskDialogIndirect: Option<TaskDialogIndirectFn>,
}

static FUNCTIONS: OnceLock<Option<ComCtlFunctions>> = OnceLock::new();

/// Returns the `comctl32.dll` function table, loading on first call.
///
/// First call must occur **after** visual-style activation to load v6. Returns `None` if DLL/required function is
/// absent.
pub fn functions() -> Option<&'static ComCtlFunctions> {
    FUNCTIONS
        .get_or_init(|| {
            let lib = super::dynamic::Library::load("comctl32.dll")?;
            // SAFETY: cada símbolo é convertido para a assinatura documentada pela
            // Microsoft para a função de mesmo nome.
            unsafe {
                Some(ComCtlFunctions {
                    InitCommonControlsEx: lib.symbol(b"InitCommonControlsEx\0")?,
                    ImageList_Create: lib.symbol(b"ImageList_Create\0")?,
                    ImageList_Destroy: lib.symbol(b"ImageList_Destroy\0")?,
                    ImageList_ReplaceIcon: lib.symbol(b"ImageList_ReplaceIcon\0")?,
                    ImageList_Add: lib.symbol(b"ImageList_Add\0")?,
                    ImageList_GetImageCount: lib.symbol(b"ImageList_GetImageCount\0")?,
                    ImageList_GetIconSize: lib.symbol(b"ImageList_GetIconSize\0")?,
                    ImageList_Draw: lib.symbol(b"ImageList_Draw\0")?,
                    TaskDialogIndirect: lib.symbol(b"TaskDialogIndirect\0"),
                })
            }
        })
        .as_ref()
}
