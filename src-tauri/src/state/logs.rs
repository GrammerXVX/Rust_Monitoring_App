use serde::Serialize;
use tokio_util::sync::CancellationToken;
use std::sync::Mutex;

pub struct LoadingState {
    pub cancel_token: Mutex<Option<CancellationToken>>,
    pub is_loading: Mutex<bool>,
}
#[derive(Serialize, Clone)]
pub struct LoadProgress {
    pub(crate) current: usize,
    pub(crate) total: usize,
}