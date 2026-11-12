//! The Settings module's Presenters: the frame's, and each config page's as it moves here
//! (General and Display, #1039).

mod about;
mod clear_logs;
pub(crate) mod data_backup;
mod device;
pub mod display;
pub mod form;
mod frame;
pub mod general;
pub mod ledger;
pub(crate) mod sync_server;
pub mod tracing;

pub use about::{AboutEvent, AboutPresenter, AboutSetup, AboutViewModel};
pub use clear_logs::{ClearLogsEvent, ClearLogsOutcome, ClearLogsPresenter, ClearLogsViewModel};
pub use data_backup::{DataBackupEvent, DataBackupPresenter, DataBackupRow, DataBackupViewModel};
pub use display::{
    ColourCardView, ColourThemeView, DisplayEvent, DisplayField, DisplayOutput, DisplayPresenter,
    DisplaySettings, DisplaySetup, DisplayViewModel, PreviewRowView, PreviewView,
};
pub use form::{
    DeviceForm, FieldDecl, FieldKind, FieldValue, FieldView, FormAnswer, FormInput, Presentation,
};
pub use frame::{
    Breadcrumb, IndexGroup, IndexRow, PickerGroup, PickerRow, PickerView, SettingsFocus,
    SettingsFrameEvent, SettingsFramePresenter, SettingsFrameSetup, SettingsFrameViewModel,
};
pub use general::{
    DataFolderOutcome, DataFolderView, GeneralEvent, GeneralField, GeneralPresenter,
    GeneralSettings, GeneralSetup, GeneralViewModel, LocaleLine, ThisDeviceView,
};
pub use ledger::{
    BaseUnitOption, BaseUnitView, LedgerEvent, LedgerPresenter, LedgerSetup, LedgerViewModel,
};
pub use sync_server::{SyncServerEvent, SyncServerPresenter, SyncServerViewModel};
pub use tracing::{
    LogChange, LogListChange, LogScroll, TracingEvent, TracingPresenter, TracingSetup,
    TracingViewModel,
};
