use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Chat {
    pub id: i64,
    pub participant_ids: [i64; 2],
    #[serde(skip_serializing, skip_deserializing)]
    pub messages: Vec<Message>,
}

impl Chat {
    pub fn new(id: i64, participant_ids: [i64; 2]) -> Self {
        Self {
            id,
            participant_ids,
            messages: Vec::new(),
        }
    }
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub source_user_id: i64,
    pub destination_user_id: i64,
    pub timestamp: i64,
    pub message: String,
}

pub type Contacts = HashMap<i64, Vec<i64>>;
