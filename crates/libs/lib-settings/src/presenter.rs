//! The Settings module's Presenters: the frame's today, and each config page's as it moves here.

mod frame;

pub use frame::{
    Breadcrumb, IndexGroup, IndexRow, PickerGroup, PickerRow, PickerView, SettingsFocus,
    SettingsFrameEvent, SettingsFramePresenter, SettingsFrameSetup, SettingsFrameViewModel,
};
