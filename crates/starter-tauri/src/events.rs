//! Bridge a `futures::Stream<Item = T>` to a `tauri::ipc::Channel<T>`
//! with per-channel cancellation.
//!
//! Tauri 2's `Channel` is the canonical way to push a sequence of events
//! to the frontend — better than the global event bus because each
//! subscription is isolated and the channel id round-trips so the
//! frontend can unsubscribe. Every desktop shell ends up writing the
//! same forwarder task and the same cancel-on-unsubscribe bookkeeping;
//! this module is that, exactly once.
//!
//! Typical wiring:
//!
//! ```ignore
//! // In your AppState:
//! pub struct AppState {
//!     pub subs: Arc<SubscriptionMap>,
//!     // ...
//! }
//!
//! #[tauri::command]
//! pub async fn subscribe(
//!     state: tauri::State<'_, AppState>,
//!     filter: MyFilter,
//!     channel: tauri::ipc::Channel<MyEvent>,
//! ) -> CommandResult<()> {
//!     let stream = state.svc.subscribe(filter).await?;
//!     starter_tauri::events::spawn_bridge(&state.subs, channel, stream);
//!     Ok(())
//! }
//!
//! #[tauri::command]
//! pub async fn unsubscribe(
//!     state: tauri::State<'_, AppState>,
//!     channel_id: u32,
//! ) -> CommandResult<()> {
//!     starter_tauri::events::cancel(&state.subs, channel_id);
//!     Ok(())
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use futures::stream::StreamExt;
use parking_lot::Mutex;
use serde::Serialize;
use tokio_util::sync::CancellationToken;

/// Live subscription registry — keyed by `tauri::ipc::Channel::id()`.
/// Wrap in `Arc` and hand to your `AppState`.
pub type SubscriptionMap = Mutex<HashMap<u32, CancellationToken>>;

/// Forward `stream` to `channel` on a background task. The task exits
/// when the stream ends, the channel send fails (frontend dropped it),
/// or [`cancel`] is called with this channel's id.
pub fn spawn_bridge<T, S>(subs: &Arc<SubscriptionMap>, channel: tauri::ipc::Channel<T>, stream: S)
where
    T: Serialize + Clone + Send + 'static,
    S: futures::Stream<Item = T> + Send + 'static,
{
    let channel_id = channel.id();
    let token = CancellationToken::new();
    subs.lock().insert(channel_id, token.clone());

    let subs = Arc::clone(subs);
    tokio::spawn(async move {
        let _guard = DropGuard { subs, channel_id };
        tokio::pin!(stream);
        loop {
            tokio::select! {
                _ = token.cancelled() => break,
                item = stream.next() => match item {
                    Some(value) => {
                        if channel.send(value).is_err() {
                            break;
                        }
                    }
                    None => break,
                },
            }
        }
    });
}

/// Cancel the subscription with this channel id, if any. Safe to call
/// for unknown ids.
pub fn cancel(subs: &Arc<SubscriptionMap>, channel_id: u32) {
    if let Some(token) = subs.lock().remove(&channel_id) {
        token.cancel();
    }
}

/// Removes the registry entry when the forwarder task exits for any
/// reason (stream end, send error, explicit cancel). Without this the
/// map would slowly fill with stale ids whose tokens nobody will ever
/// trigger.
struct DropGuard {
    subs: Arc<SubscriptionMap>,
    channel_id: u32,
}

impl Drop for DropGuard {
    fn drop(&mut self) {
        self.subs.lock().remove(&self.channel_id);
    }
}
