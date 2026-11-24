//! Validação das estruturas e constantes FFI declaradas à mão.
//!
//! Os tamanhos esperados são os de `sizeof` nos cabeçalhos do Windows SDK para
//! x86_64. Um erro aqui indicaria um campo faltando, um tipo errado ou um
//! empacotamento incorreto — exatamente os defeitos que corromperiam memória.

#![cfg(target_pointer_width = "64")]

use arqrustui::ffi::*;
use std::mem::{align_of, size_of};

macro_rules! assert_sizes {
    ($($ty:ty => $size:expr),+ $(,)?) => {
        $(assert_eq!(size_of::<$ty>(), $size, concat!("sizeof(", stringify!($ty), ")"));)+
    };
}

#[test]
fn base_types() {
    assert_sizes! {
        POINT => 8, SIZE => 8, RECT => 16, GUID => 16, SYSTEMTIME => 16,
        HWND => 8, WPARAM => 8, LPARAM => 8, LRESULT => 8, BOOL => 4, COLORREF => 4,
    }
    assert_eq!(align_of::<GUID>(), 4);
    assert_eq!(rgb(0x11, 0x22, 0x33), 0x0033_2211);
    assert_eq!((loword(0x1234_5678), hiword(0x1234_5678)), (0x5678, 0x1234));
    assert_eq!((x_lparam(0xFFFF_0005_u32 as isize), y_lparam(0xFFFF_0005_u32 as isize)), (5, -1));
    assert_eq!(make_lparam(0xFFFF, 1), 0x0001_FFFF);
    assert!(succeeded(S_OK) && succeeded(S_FALSE) && !succeeded(E_INVALIDARG));
}

#[test]
fn user32_structures() {
    use user32::*;
    assert_sizes! {
        WNDCLASSEXW => 80, MSG => 48, PAINTSTRUCT => 72, MINMAXINFO => 40, MONITORINFOEXW => 104,
        TRACKMOUSEEVENT => 24, NMHDR => 24, WINDOWPLACEMENT => 44, MENUITEMINFOW => 80,
        NONCLIENTMETRICSW => 504, HIGHCONTRASTW => 16, SCROLLINFO => 28, DRAWITEMSTRUCT => 64,
        MEASUREITEMSTRUCT => 32, BUTTON_IMAGELIST => 32, ICONINFO => 32,
    }
    assert_eq!(WS_OVERLAPPEDWINDOW, 0x00CF_0000);
    assert_eq!((WM_COMMAND, WM_NOTIFY, WM_DPICHANGED), (0x0111, 0x004E, 0x02E0));
    assert_eq!(CW_USEDEFAULT, i32::MIN);
    assert_eq!(HWND_MESSAGE as isize, -3);
    assert_eq!((BCM_SETIMAGELIST, WM_MEASUREITEM, SS_OWNERDRAW, CBS_OWNERDRAWFIXED), (0x1602, 0x002C, 0x0D, 0x10));
}

#[test]
fn gdi_structures() {
    use gdi32::*;
    assert_sizes! {
        LOGFONTW => 92, BITMAP => 32, BITMAPINFOHEADER => 40, XFORM => 24, TRIVERTEX => 16,
        GRADIENT_RECT => 8, BLENDFUNCTION => 4, DOCINFOW => 40,
    }
}

#[test]
fn common_control_structures() {
    use comctl32::*;
    assert_sizes! {
        INITCOMMONCONTROLSEX => 8, LVCOLUMNW => 56, LVITEMW => 88, NMLISTVIEW => 64,
        NMLVDISPINFOW => 112, LVHITTESTINFO => 24, TVITEMW => 56, TVITEMEXW => 80,
        TVINSERTSTRUCTW => 96, NMTREEVIEWW => 152, TCITEMW => 40, TBBUTTON => 32,
        TTTOOLINFOW => 72, NMCUSTOMDRAW => 80, NMLVCUSTOMDRAW => 136, HDITEMW => 72, NMHEADERW => 40,
        REBARBANDINFOW => 128, NMUPDOWN => 32, NMPGCALCSIZE => 40, NMLINK => 4304,
        CHARFORMAT2W => 116, PARAFORMAT => 156, PARAFORMAT2 => 188, EDITSTREAM => 20,
        TASKDIALOG_BUTTON => 12, TASKDIALOGCONFIG => 160,
    }
    assert_eq!((LVN_ITEMCHANGED, TVN_SELCHANGEDW, TCN_SELCHANGE, NM_CUSTOMDRAW), (-101, -451, -551, -12));
    assert_eq!(TVI_ROOT as isize, -0x10000);
}

#[test]
fn dialog_shell_and_misc_structures() {
    assert_sizes! {
        comdlg32::CHOOSECOLORW => 72, comdlg32::CHOOSEFONTW => 104, comdlg32::PRINTDLGW => 120,
        comdlg32::PAGESETUPDLGW => 128, comdlg32::FINDREPLACEW => 80,
        shell32::NOTIFYICONDATAW => 976, shell32::COMDLG_FILTERSPEC => 16,
        kernel32::ACTCTXW => 56, user32::GESTUREINFO => 56, user32::GESTURECONFIG => 12, kernel32::OSVERSIONINFOW => 276,
        ole32::VARIANT => 24, dwmapi::MARGINS => 16, imm32::COMPOSITIONFORM => 28, imm32::CANDIDATELIST => 28,
        gdiplus::GdiplusStartupInput => 24, gdiplus::PointF => 8, gdiplus::RectF => 16,
        d2d1::D2D1_COLOR_F => 16, d2d1::D2D1_RECT_F => 16, d2d1::D2D1_ROUNDED_RECT => 24,
        d2d1::D2D1_ELLIPSE => 16, d2d1::D2D1_RENDER_TARGET_PROPERTIES => 28,
        d2d1::D2D1_HWND_RENDER_TARGET_PROPERTIES => 24, d2d1::D2D1_GRADIENT_STOP => 20,
        d2d1::D2D1_BEZIER_SEGMENT => 24,
    }
    assert_eq!(shell32::IID_IShellItem.data1, 0x43826D1E);
    assert_eq!(d2d1::D2DERR_RECREATE_TARGET as u32, 0x8899_000C);
}

#[test]
fn optional_apis_resolve_without_crashing() {
    // Cada entrada pode existir ou não conforme a versão do sistema; o que se exige
    // é que a detecção nunca falhe e que comctl32 esteja disponível.
    let optional = dynamic::optional();
    let _ = (optional.GetDpiForWindow.is_some(), optional.GetDpiForMonitor.is_some());
    assert!(dynamic::Library::load("user32.dll").is_some());
    assert!(dynamic::Library::load("dll-que-nao-existe-arqrustui.dll").is_none());
    assert!(comctl32::functions().is_some());
}
