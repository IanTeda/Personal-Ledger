//! The Settings module's Presenters: the frame's, and each config page's as it moves here
//! (General and Display, #1039).

mod clear_logs;
mod device;
pub mod display;
mod frame;
pub mod general;
pub mod tracing;

pub use clear_logs::{ClearLogsOutcome, ClearLogsPresenter, ClearLogsViewModel};
pub use device::{ControlView, PageKey};
pub use display::{
    CheckboxView, ColourCardView, ColourThemeView, DisplayEvent, DisplayOutput, DisplayPresenter,
    DisplaySettings, DisplaySetup, DisplayViewModel, PreviewRowView, PreviewView,
};
pub use frame::{
    Breadcrumb, IndexGroup, IndexRow, PickerGroup, PickerRow, PickerView, SettingsFocus,
    SettingsFrameEvent, SettingsFramePresenter, SettingsFrameSetup, SettingsFrameViewModel,
};
pub use general::{
    GeneralEvent, GeneralPresenter, GeneralSettings, GeneralSetup, GeneralViewModel, LocaleLine,
    ReadOnlyView, StartViewControl, ThisDevice, ThisDeviceView,
};
pub use tracing::{
    LogChange, LogListChange, LogScroll, TracingEvent, TracingPresenter, TracingSetup,
    TracingViewModel,
};
