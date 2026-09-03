//! Bounded activity state for operations and agent notifications.
//!
//! Operations and notifications are observations of the authenticated agent;
//! they are never copied into a second management model. Each feed is
//! reconnecting and keeps only the current session's bounded history.

use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;

use msc_api::dto::{NotificationEventDto, OperationDto, SimpleResultDto};

use super::overview::OverviewState;
use super::transport::{SharedClient, StreamChannel, reconnect_delay};
use crate::cli::CliError;

pub const OPERATION_HISTORY_LIMIT: usize = 64;
pub const NOTIFICATION_HISTORY_LIMIT: usize = 200;

#[derive(Debug)]
struct ActivityFeed {
    receiver: Mutex<Receiver<FeedEvent>>,
    sender: mpsc::Sender<FeedEvent>,
}

#[derive(Debug)]
enum FeedEvent {
    Operation(OperationDto),
    Notification(NotificationEventDto),
    Status(String),
}

#[derive(Debug, Clone, Default)]
pub struct ActivityState {
    operations: VecDeque<OperationDto>,
    notifications: VecDeque<NotificationEventDto>,
    feed: Option<Arc<ActivityFeed>>,
    notifications_started: bool,
    tracked_operations: Vec<String>,
    selected_index: usize,
    open: bool,
    status: Option<String>,
}

impl ActivityState {
    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn operations(&self) -> impl Iterator<Item = &OperationDto> {
        self.operations.iter()
    }

    pub fn notifications(&self) -> impl Iterator<Item = &NotificationEventDto> {
        self.notifications.iter()
    }

    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn move_selection(&mut self, offset: isize) {
        let count = self.operations.len() + self.notifications.len();
        if count == 0 {
            self.selected_index = 0;
            return;
        }
        self.selected_index =
            (self.selected_index as isize + offset).rem_euclid(count as isize) as usize;
    }

    pub fn selected_operation_id(&self) -> Option<&str> {
        self.operations
            .get(self.selected_index)
            .map(|operation| operation.id.as_str())
    }

    pub fn selected_operation_is_active(&self) -> bool {
        self.operations
            .get(self.selected_index)
            .is_some_and(|operation| {
                matches!(
                    operation.state,
                    msc_api::dto::OperationStateDto::Queued
                        | msc_api::dto::OperationStateDto::Running
                )
            })
    }

    /// Start the existing notification channel once for this host session.
    /// The channel is agent-owned; this client does not create a TUI-only
    /// event route or event shape.
    pub fn start_notifications(&mut self, client: SharedClient) {
        if self.notifications_started {
            return;
        }
        self.notifications_started = true;
        let sender = self.ensure_feed();
        thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("notification feed runtime builds");
            runtime.block_on(run_notifications(client, sender));
        });
    }

    /// Track an operation returned by an existing mutating route. A terminal
    /// snapshot is still retained so the activity surface explains the final
    /// result after the server closes the stream normally.
    pub fn track_operation(&mut self, client: SharedClient, operation_id: impl Into<String>) {
        let operation_id = operation_id.into();
        if operation_id.is_empty() || self.tracked_operations.contains(&operation_id) {
            return;
        }
        self.tracked_operations.push(operation_id.clone());
        let sender = self.ensure_feed();
        thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("operation feed runtime builds");
            runtime.block_on(run_operation(client, operation_id, sender));
        });
    }

    pub fn poll(&mut self) {
        let Some(feed) = self.feed.clone() else {
            return;
        };
        loop {
            let event = feed
                .receiver
                .lock()
                .expect("activity feed lock poisoned")
                .try_recv();
            match event {
                Ok(FeedEvent::Operation(operation)) => self.record_operation(operation),
                Ok(FeedEvent::Notification(notification)) => self.record_notification(notification),
                Ok(FeedEvent::Status(status)) => self.status = Some(status),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
    }

    pub async fn request_lifecycle(
        client: &SharedClient,
        start: bool,
    ) -> Result<(SimpleResultDto, OverviewState), CliError> {
        let path = if start { "/v1/start" } else { "/v1/stop" };
        let result: SimpleResultDto = client.post_json(path, &serde_json::json!({})).await?;
        let overview = OverviewState::load(client).await?;
        Ok((result, overview))
    }

    pub async fn cancel_operation(
        client: &SharedClient,
        operation_id: &str,
    ) -> Result<OperationDto, CliError> {
        client
            .post_json(
                &format!("/v1/operations/{operation_id}/cancel"),
                &serde_json::json!({}),
            )
            .await
    }

    fn ensure_feed(&mut self) -> mpsc::Sender<FeedEvent> {
        if let Some(feed) = &self.feed {
            // The sender is kept beside the receiver so a later operation can
            // join the same session feed without opening a second channel.
            return feed_sender(feed);
        }
        let (sender, receiver) = mpsc::channel();
        self.feed = Some(Arc::new(ActivityFeed {
            receiver: Mutex::new(receiver),
            sender: sender.clone(),
        }));
        sender
    }

    fn record_operation(&mut self, operation: OperationDto) {
        if let Some(existing) = self
            .operations
            .iter_mut()
            .find(|existing| existing.id == operation.id)
        {
            *existing = operation;
        } else {
            self.operations.push_back(operation);
            while self.operations.len() > OPERATION_HISTORY_LIMIT {
                self.operations.pop_front();
            }
        }
        self.selected_index = self.selected_index.min(self.item_count().saturating_sub(1));
    }

    #[cfg(test)]
    pub fn accept_operation_for_test(&mut self, operation: OperationDto) {
        self.record_operation(operation);
    }

    #[cfg(test)]
    pub fn accept_notification_for_test(&mut self, notification: NotificationEventDto) {
        self.record_notification(notification);
    }

    fn record_notification(&mut self, notification: NotificationEventDto) {
        if self
            .notifications
            .iter()
            .any(|existing| existing.id == notification.id)
        {
            return;
        }
        self.notifications.push_back(notification);
        while self.notifications.len() > NOTIFICATION_HISTORY_LIMIT {
            self.notifications.pop_front();
        }
        self.selected_index = self.selected_index.min(self.item_count().saturating_sub(1));
    }

    fn item_count(&self) -> usize {
        self.operations.len() + self.notifications.len()
    }
}

fn feed_sender(feed: &ActivityFeed) -> mpsc::Sender<FeedEvent> {
    feed.sender.clone()
}

async fn run_notifications(client: SharedClient, sender: mpsc::Sender<FeedEvent>) {
    let mut attempt: u32 = 0;
    loop {
        match client.open_stream(StreamChannel::Notifications).await {
            Ok(mut stream) => {
                attempt = 0;
                let _ = sender.send(FeedEvent::Status("Notification stream connected".into()));
                while let Ok(Some(notification)) = stream.next_json::<NotificationEventDto>().await
                {
                    if sender.send(FeedEvent::Notification(notification)).is_err() {
                        return;
                    }
                }
            }
            Err(error) => {
                attempt = attempt.saturating_add(1);
                let _ = sender.send(FeedEvent::Status(format!(
                    "Notification stream reconnecting: {error}"
                )));
            }
        }
        tokio::time::sleep(reconnect_delay(attempt.max(1))).await;
    }
}

async fn run_operation(
    client: SharedClient,
    operation_id: String,
    sender: mpsc::Sender<FeedEvent>,
) {
    let mut attempt: u32 = 0;
    loop {
        match client
            .open_stream(StreamChannel::Operation {
                id: operation_id.clone(),
            })
            .await
        {
            Ok(mut stream) => {
                attempt = 0;
                loop {
                    match stream.next_json::<OperationDto>().await {
                        Ok(Some(operation)) => {
                            let terminal = operation.state.is_terminal();
                            if sender.send(FeedEvent::Operation(operation)).is_err() {
                                return;
                            }
                            if terminal {
                                return;
                            }
                        }
                        // A terminal operation closes normally. The stream
                        // transport marks that close as None, so it must not
                        // be reported as a reconnect failure.
                        Ok(None) => return,
                        Err(error) => {
                            let _ = sender.send(FeedEvent::Status(format!(
                                "Operation {operation_id} reconnecting: {error}"
                            )));
                            break;
                        }
                    }
                }
            }
            Err(error) => {
                attempt = attempt.saturating_add(1);
                let _ = sender.send(FeedEvent::Status(format!(
                    "Operation {operation_id} reconnecting: {error}"
                )));
            }
        }
        tokio::time::sleep(reconnect_delay(attempt.max(1))).await;
    }
}

trait TerminalOperation {
    fn is_terminal(&self) -> bool;
}

impl TerminalOperation for msc_api::dto::OperationStateDto {
    fn is_terminal(&self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}
