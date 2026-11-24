//! Accessibility constants/GUIDs for `uiautomationcore.dll` and `oleacc.dll`.
//!
//! `Uia*` functions resolve dynamically in [`super::dynamic`].

use super::types::*;

pub const UIA_ROOT_OBJECT_ID: i32 = -25;
pub const PROVIDER_OPTIONS_SERVER_SIDE: i32 = 0x1;
pub const UIA_CONTROL_TYPE_PROPERTY_ID: i32 = 30003;
pub const UIA_NAME_PROPERTY_ID: i32 = 30005;
pub const UIA_HELP_TEXT_PROPERTY_ID: i32 = 30013;
pub const UIA_IS_CONTROL_ELEMENT_PROPERTY_ID: i32 = 30016;
pub const UIA_IS_CONTENT_ELEMENT_PROPERTY_ID: i32 = 30017;
pub const UIA_IS_KEYBOARD_FOCUSABLE_PROPERTY_ID: i32 = 30009;
pub const UIA_HAS_KEYBOARD_FOCUS_PROPERTY_ID: i32 = 30008;
pub const UIA_IS_ENABLED_PROPERTY_ID: i32 = 30010;
pub const UIA_LOCALIZED_CONTROL_TYPE_PROPERTY_ID: i32 = 30004;
pub const UIA_FULL_DESCRIPTION_PROPERTY_ID: i32 = 30159;

pub const IID_IRawElementProviderSimple: GUID =
    GUID::new(0xD6DD68D1, 0x86FD, 0x4332, [0x86, 0x66, 0x9A, 0xBE, 0xDE, 0xA2, 0xD2, 0x4C]);
pub const CLSID_AccPropServices: GUID =
    GUID::new(0xB5F8350B, 0x0548, 0x48B1, [0xA6, 0xEE, 0x88, 0xBD, 0x00, 0xB4, 0xA5, 0xE7]);
pub const IID_IAccPropServices: GUID =
    GUID::new(0x6E26E776, 0x04F0, 0x495D, [0x80, 0xE4, 0x33, 0x30, 0x35, 0x2E, 0x31, 0x69]);
pub const PROPID_ACC_NAME: GUID =
    GUID::new(0x608D3DF8, 0x8128, 0x4AA7, [0xA4, 0x28, 0xF5, 0x5E, 0x49, 0x26, 0x72, 0x91]);
pub const PROPID_ACC_DESCRIPTION: GUID =
    GUID::new(0x4D48DFE4, 0xBD3F, 0x491F, [0xA6, 0x48, 0x49, 0x2D, 0x6F, 0x20, 0xC5, 0x88]);
pub const PROPID_ACC_HELP: GUID =
    GUID::new(0xC831E11F, 0x44DB, 0x4A99, [0x97, 0x68, 0xCB, 0x8F, 0x97, 0x8B, 0x72, 0x31]);

pub const PROVIDER_OPTIONS_USE_COM_THREADING: i32 = 0x20;
pub const UIA_APPEND_RUNTIME_ID: i32 = 3;
pub const UIA_E_ELEMENTNOTAVAILABLE: HRESULT = 0x8004_0201_u32 as HRESULT;
pub const UIA_E_ELEMENTNOTENABLED: HRESULT = 0x8004_0200_u32 as HRESULT;
pub const UIA_E_INVALIDOPERATION: HRESULT = 0x8013_1509_u32 as HRESULT;

pub const UIA_INVOKE_PATTERN_ID: i32 = 10000;
pub const UIA_VALUE_PATTERN_ID: i32 = 10002;
pub const UIA_EXPAND_COLLAPSE_PATTERN_ID: i32 = 10005;
pub const UIA_TOGGLE_PATTERN_ID: i32 = 10015;

pub const UIA_MENU_OPENED_EVENT_ID: i32 = 20003;
pub const UIA_AUTOMATION_FOCUS_CHANGED_EVENT_ID: i32 = 20005;
pub const UIA_MENU_CLOSED_EVENT_ID: i32 = 20007;
pub const UIA_INVOKE_INVOKED_EVENT_ID: i32 = 20009;

pub const UIA_ACCELERATOR_KEY_PROPERTY_ID: i32 = 30006;
pub const UIA_ACCESS_KEY_PROPERTY_ID: i32 = 30007;
pub const UIA_AUTOMATION_ID_PROPERTY_ID: i32 = 30011;
pub const UIA_CLASS_NAME_PROPERTY_ID: i32 = 30012;
pub const UIA_IS_OFFSCREEN_PROPERTY_ID: i32 = 30022;
pub const UIA_FRAMEWORK_ID_PROPERTY_ID: i32 = 30024;
pub const UIA_VALUE_VALUE_PROPERTY_ID: i32 = 30045;
pub const UIA_VALUE_IS_READ_ONLY_PROPERTY_ID: i32 = 30046;
pub const UIA_EXPAND_COLLAPSE_STATE_PROPERTY_ID: i32 = 30070;
pub const UIA_TOGGLE_STATE_PROPERTY_ID: i32 = 30086;
pub const UIA_POSITION_IN_SET_PROPERTY_ID: i32 = 30152;
pub const UIA_SIZE_OF_SET_PROPERTY_ID: i32 = 30153;

pub const IID_IRawElementProviderFragment: GUID =
    GUID::new(0xF7063DA8, 0x8359, 0x439C, [0x92, 0x97, 0xBB, 0xC5, 0x29, 0x9A, 0x7D, 0x87]);
pub const IID_IRawElementProviderFragmentRoot: GUID =
    GUID::new(0x620CE2A5, 0xAB8F, 0x40A9, [0x86, 0xCB, 0xDE, 0x3C, 0x75, 0x59, 0x9B, 0x58]);
pub const IID_IInvokeProvider: GUID =
    GUID::new(0x54FCB24B, 0xE18E, 0x47A2, [0xB4, 0xD3, 0xEC, 0xCB, 0xE7, 0x75, 0x99, 0xA2]);
pub const IID_IToggleProvider: GUID =
    GUID::new(0x56D00BD0, 0xC4F4, 0x433C, [0xA8, 0x36, 0x1A, 0x52, 0xA5, 0x7E, 0x08, 0x92]);
pub const IID_IValueProvider: GUID =
    GUID::new(0xC7935180, 0x6FB3, 0x4201, [0xB1, 0x74, 0x7D, 0xF7, 0x3A, 0xDB, 0xF6, 0x4A]);
pub const IID_IExpandCollapseProvider: GUID =
    GUID::new(0xD847D3A5, 0xCAB0, 0x4A98, [0x8C, 0x32, 0xEC, 0xB4, 0x5C, 0x59, 0xAD, 0x24]);

/// `UiaRect`: screen pixel rectangle with floating-point coordinates.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UiaRect {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

// Padrões e propriedades adicionais (UIAutomationClient.h / UIAutomationCore.h).
pub const UIA_SELECTION_PATTERN_ID: i32 = 10001;
pub const UIA_RANGE_VALUE_PATTERN_ID: i32 = 10003;
pub const UIA_SCROLL_PATTERN_ID: i32 = 10004;
pub const UIA_GRID_PATTERN_ID: i32 = 10006;
pub const UIA_GRID_ITEM_PATTERN_ID: i32 = 10007;
pub const UIA_SELECTION_ITEM_PATTERN_ID: i32 = 10010;
pub const UIA_TEXT_PATTERN_ID: i32 = 10014;

pub const UIA_STRUCTURE_CHANGED_EVENT_ID: i32 = 20002;
pub const UIA_AUTOMATION_PROPERTY_CHANGED_EVENT_ID: i32 = 20004;
pub const UIA_SELECTION_ITEM_ELEMENT_SELECTED_EVENT_ID: i32 = 20012;
pub const UIA_TEXT_TEXT_SELECTION_CHANGED_EVENT_ID: i32 = 20014;
pub const UIA_TEXT_TEXT_CHANGED_EVENT_ID: i32 = 20015;
pub const UIA_LIVE_REGION_CHANGED_EVENT_ID: i32 = 20024;

pub const UIA_RANGE_VALUE_VALUE_PROPERTY_ID: i32 = 30047;
pub const UIA_RANGE_VALUE_IS_READ_ONLY_PROPERTY_ID: i32 = 30048;
pub const UIA_RANGE_VALUE_MINIMUM_PROPERTY_ID: i32 = 30049;
pub const UIA_RANGE_VALUE_MAXIMUM_PROPERTY_ID: i32 = 30050;
pub const UIA_RANGE_VALUE_LARGE_CHANGE_PROPERTY_ID: i32 = 30051;
pub const UIA_RANGE_VALUE_SMALL_CHANGE_PROPERTY_ID: i32 = 30052;
pub const UIA_SCROLL_HORIZONTAL_SCROLL_PERCENT_PROPERTY_ID: i32 = 30053;
pub const UIA_SCROLL_HORIZONTAL_VIEW_SIZE_PROPERTY_ID: i32 = 30054;
pub const UIA_SCROLL_VERTICAL_SCROLL_PERCENT_PROPERTY_ID: i32 = 30055;
pub const UIA_SCROLL_VERTICAL_VIEW_SIZE_PROPERTY_ID: i32 = 30056;
pub const UIA_SCROLL_HORIZONTALLY_SCROLLABLE_PROPERTY_ID: i32 = 30057;
pub const UIA_SCROLL_VERTICALLY_SCROLLABLE_PROPERTY_ID: i32 = 30058;
pub const UIA_SELECTION_CAN_SELECT_MULTIPLE_PROPERTY_ID: i32 = 30060;
pub const UIA_SELECTION_IS_SELECTION_REQUIRED_PROPERTY_ID: i32 = 30061;
pub const UIA_GRID_ROW_COUNT_PROPERTY_ID: i32 = 30062;
pub const UIA_GRID_COLUMN_COUNT_PROPERTY_ID: i32 = 30063;
pub const UIA_GRID_ITEM_ROW_PROPERTY_ID: i32 = 30064;
pub const UIA_GRID_ITEM_COLUMN_PROPERTY_ID: i32 = 30065;
pub const UIA_SELECTION_ITEM_IS_SELECTED_PROPERTY_ID: i32 = 30079;
pub const UIA_LIVE_SETTING_PROPERTY_ID: i32 = 30135;

pub const IID_ISelectionProvider: GUID =
    GUID::new(0xFB8B03AF, 0x3BDF, 0x48D4, [0xBD, 0x36, 0x1A, 0x65, 0x79, 0x3B, 0xE1, 0x68]);
pub const IID_ISelectionItemProvider: GUID =
    GUID::new(0x2ACAD808, 0xB2D4, 0x452D, [0xA4, 0x07, 0x91, 0xFF, 0x1A, 0xD1, 0x67, 0xB2]);
pub const IID_IScrollProvider: GUID =
    GUID::new(0xB38B8077, 0x1FC3, 0x42A5, [0x8C, 0xAE, 0xD4, 0x0C, 0x22, 0x15, 0x05, 0x5A]);
pub const IID_IGridProvider: GUID =
    GUID::new(0xB17D6187, 0x0907, 0x464B, [0xA1, 0x68, 0x0E, 0xF1, 0x7A, 0x15, 0x72, 0xB1]);
pub const IID_IGridItemProvider: GUID =
    GUID::new(0xD02541F1, 0xFB81, 0x4D64, [0xAE, 0x32, 0xF5, 0x20, 0xF8, 0xA6, 0xDB, 0xD1]);
pub const IID_IRangeValueProvider: GUID =
    GUID::new(0x36DC7AEF, 0x33E6, 0x4691, [0xAF, 0xE1, 0x2B, 0xE7, 0x27, 0x4B, 0x3D, 0x33]);
pub const IID_ITextProvider: GUID =
    GUID::new(0x3589C92C, 0x63F3, 0x4367, [0x99, 0xBB, 0xAD, 0xA6, 0x53, 0xB7, 0x7C, 0xF2]);
pub const IID_ITextRangeProvider: GUID =
    GUID::new(0x5347AD7B, 0xC355, 0x46F8, [0xAF, 0xF5, 0x90, 0x90, 0x33, 0x58, 0x2F, 0x63]);
pub const IID_IRawElementProviderAdviseEvents: GUID =
    GUID::new(0xA407B27B, 0x0F6D, 0x4427, [0x92, 0x92, 0x47, 0x3C, 0x7B, 0xF9, 0x32, 0x58]);

/// `UiaPoint`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UiaPoint {
    pub x: f64,
    pub y: f64,
}
