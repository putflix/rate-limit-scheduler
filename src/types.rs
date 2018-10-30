use serde_json::Value;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PollQueueItemsResponse {
    pub items: Vec<QueueItem>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct QueueItem {
    pub endpoint: String,
    pub id: String,
    pub payload: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct QueueItemFeedback {
    pub id: String,
    pub status: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct QueueItemFeedbackRequest {
    pub items: Vec<QueueItemFeedback>,
}
