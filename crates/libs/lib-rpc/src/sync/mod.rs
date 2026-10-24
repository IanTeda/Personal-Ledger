pub(crate) mod proto;

pub use proto::{
    ChangeSet, PullRequest, PullResponse, PushRequest, PushResponse, SyncService,
    SyncServiceClient, SyncServiceServer,
};
