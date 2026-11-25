//! Settings as a VIPER module (`docs/development/viper-architecture.md`): the frame's Presenter,
//! view model and keymap, [`SettingsPage`], the pages and their This device / This Ledger
//! grouping, and the config pages' Presenters as each moves here (General and Display, #1039).
//! Toolkit-free and I/O-free, it depends on no feature crate, nor on `lib-config`, `lib-tracing`
//! or `lib-units`: the config pages reach Configuration, the log and the Units through its own
//! ports, such as [`DeviceFile`] (#1036).
//!
//! The domain pages (Units, Institutions, Accounts, …) stay in their own modules; the frame only
//! chooses which page shows and where focus sits. A page's icon is the Client's (ADR-0037,
//! 2026-10-30).

#[cfg(any(test, feature = "contract-tests"))]
#[expect(
    clippy::panic,
    reason = "a contract check is test code a consumer runs, and fails by panicking as a test does"
)]
pub mod contract;
pub mod device;
mod error;
pub mod fake;
pub mod interactor;
pub mod keymap;
pub mod log;
pub mod msg;
pub mod page;
pub mod presenter;
pub mod protocols;
pub mod summary;

pub use crate::device::{ChangeLog, DeviceFile, DevicePorts, snapshot};
pub use crate::error::{Error, Result};
pub use crate::interactor::{
    ChooseBaseUnit, RemoveApiKeys, RemoveSetting, RenameLedger, RestoreSetting, RestoreSettings,
    Restored, SetFinancialYearStart, SetLedgerOwner, WriteSetting,
};
pub use crate::log::{LogLevel, LogLine, TracingLevel};
pub use crate::page::{ScopeFacts, ScopeGroup, SettingsPage};
pub use crate::presenter::{
    AboutEvent, AboutPresenter, AboutSetup, AboutViewModel, BaseUnitOption, BaseUnitView,
    Breadcrumb, ClearLogsEvent, ClearLogsOutcome, ClearLogsPresenter, ClearLogsViewModel,
    DataBackupEvent, DataBackupPresenter, DataBackupRow, DataBackupSetup, DataBackupViewModel,
    DataFolderOutcome, DisplayEvent, DisplayField, DisplayOutput, DisplayPresenter,
    DisplaySettings, DisplaySetup, DisplayViewModel, GeneralEvent, GeneralField, GeneralPresenter,
    GeneralSettings, GeneralSetup, GeneralViewModel, IndexGroup, IndexRow, LedgerEvent,
    LedgerField, LedgerPresenter, LedgerSetup, LedgerViewModel, LocaleLine, LogChange,
    LogListChange, LogScroll, PickerGroup, PickerRow, PickerView, RemoveApiKeysEvent,
    RemoveApiKeysOutcome, RemoveApiKeysPresenter, RemoveApiKeysViewModel, SettingsFocus,
    SettingsFrameEvent, SettingsFramePresenter, SettingsFrameSetup, SettingsFrameViewModel,
    SyncServerEvent, SyncServerPresenter, SyncServerViewModel, TracingEvent, TracingPresenter,
    TracingSetup, TracingViewModel, device_status,
};
pub use crate::protocols::{
    ApiKeys, ColourChoice, DataBackupRouter, DeviceAnswer, DeviceRefusal, DeviceRouter,
    DeviceSnapshot, HeadlineAmount, KeyState, LedgerCounts, LedgerFigures, LedgerPreferences,
    LedgerReads, LedgerRouter, LedgerSummaries, SettingsLedger, SettingsRouter, TracingLog,
    TracingRouter,
};
pub use crate::summary::{PageRows, PageSummary, SummaryRow, SummarySources, summarise};
