use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayMessage {
    pub id: String,
    pub topic: String,
    pub payload: String,
    pub sender_id: String,
    pub created_at: i64,
}

pub struct RelayStore {
    messages: HashMap<String, Vec<(Instant, RelayMessage)>>,
    ttl: Duration,
}

impl RelayStore {
    pub fn new(ttl: Duration) -> Self {
        Self {
            messages: HashMap::new(),
            ttl,
        }
    }

    pub fn push(&mut self, topic: String, message: RelayMessage) {
        self.cleanup();
        let now = Instant::now();
        self.messages.entry(topic).or_default().push((now, message));
    }

    pub fn pull(&mut self, topic: &str) -> Vec<RelayMessage> {
        self.cleanup();
        if let Some(list) = self.messages.remove(topic) {
            list.into_iter().map(|(_, msg)| msg).collect()
        } else {
            Vec::new()
        }
    }

    pub fn peek(&mut self, topic: &str) -> Vec<RelayMessage> {
        self.cleanup();
        if let Some(list) = self.messages.get(topic) {
            list.iter().map(|(_, msg)| msg.clone()).collect()
        } else {
            Vec::new()
        }
    }

    fn cleanup(&mut self) {
        let now = Instant::now();
        let ttl = self.ttl;
        for list in self.messages.values_mut() {
            list.retain(|(inst, _)| now.duration_since(*inst) < ttl);
        }
        self.messages.retain(|_, list| !list.is_empty());
    }
}
