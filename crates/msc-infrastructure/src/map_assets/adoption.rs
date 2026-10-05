//! A failed or superseded preparation cannot replace a usable scene.
use std::collections::BTreeMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone)]
pub struct Ticket {
    pub key: String,
    pub revision: String,
    serial: u64,
    cancelled: Arc<AtomicBool>,
}
impl Ticket {
    pub fn cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}
pub struct Entry<T> {
    pub revision: String,
    pub current: Option<Arc<T>>,
    pub operation_id: Option<String>,
    pub outcome: String,
    serial: u64,
    cancelled: Arc<AtomicBool>,
}
pub struct Coordinator<T> {
    pub entries: BTreeMap<String, Entry<T>>,
    serial: u64,
}
impl<T> Default for Coordinator<T> {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
            serial: 0,
        }
    }
}
impl<T> Coordinator<T> {
    pub fn begin(
        &mut self,
        key: &str,
        revision: &str,
        operation: &str,
        force: bool,
    ) -> Option<Ticket> {
        if let Some(entry) = self.entries.get(key) {
            if !force && entry.revision == revision {
                return None;
            }
            entry.cancelled.store(true, Ordering::Release);
        }
        self.serial += 1;
        let cancel = Arc::new(AtomicBool::new(false));
        let current = self.entries.get(key).and_then(|e| e.current.clone());
        self.entries.insert(
            key.into(),
            Entry {
                revision: revision.into(),
                current,
                operation_id: Some(operation.into()),
                outcome: "preparing".into(),
                serial: self.serial,
                cancelled: cancel.clone(),
            },
        );
        Some(Ticket {
            key: key.into(),
            revision: revision.into(),
            serial: self.serial,
            cancelled: cancel,
        })
    }
    pub fn matches(&self, ticket: &Ticket) -> bool {
        !ticket.cancelled()
            && self
                .entries
                .get(&ticket.key)
                .is_some_and(|e| e.serial == ticket.serial && e.revision == ticket.revision)
    }
    /// Caller validates context and artifacts first; publication itself is a short mutex-held swap.
    pub fn finish(&mut self, ticket: &Ticket, candidate: Option<Arc<T>>, outcome: &str) -> bool {
        if !self.matches(ticket) {
            return false;
        }
        let entry = self.entries.get_mut(&ticket.key).expect("matched entry");
        if let Some(candidate) = candidate {
            entry.current = Some(candidate);
        }
        entry.outcome = outcome.into();
        true
    }
    /// Record cancellation/failure only for the same job, without adopting bytes.
    pub fn abort(&mut self, ticket: &Ticket, outcome: &str) -> bool {
        let Some(entry) = self
            .entries
            .get_mut(&ticket.key)
            .filter(|e| e.serial == ticket.serial && e.revision == ticket.revision)
        else {
            return false;
        };
        entry.cancelled.store(true, Ordering::Release);
        entry.outcome = outcome.into();
        true
    }
    pub fn cancel(&mut self, key: &str) {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.cancelled.store(true, Ordering::Release);
            entry.outcome = "cancelled".into();
        }
    }
}
