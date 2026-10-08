//! Calls the exports forward. This module is not itself exported, so a body
//! here may do more than one thing.

use crate::{FfiError, ThreadRecord, ThreadSink};

/// The protocol crate's readiness check, as the async export's one call.
pub async fn core_ready() -> Result<(), FfiError> {
    mailune_protocol::ready().map_err(|err| FfiError::NotReady {
        message: err.to_string(),
    })
}

/// Hands already-built records to the foreign callback.
pub fn publish_threads(threads: Vec<ThreadRecord>, sink: Box<dyn ThreadSink>) {
    sink.on_threads(threads);
}
