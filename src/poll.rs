use futures::{prelude::*, future, stream, sync::mpsc};
use httpdate::parse_http_date;
use reqwest::{
    async::{Client, Response},
    header::{AUTHORIZATION, RETRY_AFTER},
};
use std::{
    io::{Error, ErrorKind},
    time::{Duration, Instant, SystemTime},
};
use tokio::{self, timer::Delay};
use types::{
    PollQueueItemsResponse,
    QueueItem,
    QueueItemFeedback,
    QueueItemFeedbackRequest,
};

/// A set of configuration options for running the rate limited scheduler.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunCfg {
    pub auth_token: String,
    pub max_inflight: usize,
    pub remote_url: String,
}

/// Run the rate limit scheduler.
pub fn run(cfg: RunCfg) {
    let client = Client::new();
    let (item_tx, item_rx) = mpsc::channel(cfg.max_inflight); // Use a bounded channel for backpressure
    let (feedback_tx, feedback_rx) = mpsc::channel(cfg.max_inflight);

    let fetcher = fetch_urls(cfg.clone(), client.clone(), item_tx);
    let firerer = fire_requests(cfg.clone(), client.clone(), item_rx, feedback_tx);
    let feedback_reporter = report_feedback(cfg, client, feedback_rx);

    let run = fetcher.join3(firerer, feedback_reporter)
        .map(|_| ())
        .map_err(|e| panic!("Fatal Error: {:?}", e));

    tokio::run(run);
}

/// Continuously fetch queue items from the dispatch queue and push them into
/// the given channel. Channel backpressure will regulate the fetching speed.
fn fetch_urls(
    cfg: RunCfg,
    client: Client,
    tx: mpsc::Sender<QueueItem>,
) -> impl Future<Item = (), Error = Error> {
    stream::repeat(())
        .and_then(move |_| {
            client.get(&cfg.remote_url)
                .header(AUTHORIZATION, format!("Bearer {}", cfg.auth_token))
                .query(&[("limit", cfg.max_inflight.to_string())])
                .send()
        })
        .and_then(|res| res.error_for_status())
        .and_then(|mut res| res.json::<PollQueueItemsResponse>())
        .map_err(|e| Error::new(ErrorKind::InvalidData, e))
        .map(|data| stream::iter_ok(data.items))
        .flatten()
        .forward(tx.sink_map_err(|e| Error::new(ErrorKind::BrokenPipe, e)))
        .map(|_| ())
}

/// Fire off requests taken from the queue and report feedback to the queue server.
fn fire_requests(
    cfg: RunCfg,
    client: Client,
    item_rx: mpsc::Receiver<QueueItem>,
    feedback_tx: mpsc::Sender<QueueItemFeedback>,
) -> impl Future<Item = (), Error = Error> {
    let process_item = move |queue_item: QueueItem| {
        let cloned_feedback_tx = feedback_tx.clone();

        let req = if let Some(pl) = queue_item.payload.as_ref() {
            client.post(&queue_item.endpoint).json(pl)
        } else {
            client.get(&queue_item.endpoint)
        };

        // Send request and process response
        req.send()
            .map_err(|e| Error::new(ErrorKind::ConnectionRefused, e))
            .and_then(move |resp: Response| {
                // If the server has given us a 429 this means we have reached a rate
                // limit and need to back down. Otherwise just pass the response status
                // code through to the queue server.
                if resp.status().as_u16() != 429 {
                    return future::Either::A(
                        cloned_feedback_tx.send(QueueItemFeedback {
                            id: queue_item.id,
                            status: resp.status().as_u16(),
                        })
                            .map(|_| ())
                            .map_err(|e| Error::new(ErrorKind::BrokenPipe, e))
                    );
                }

                // Try to get the delay and wait accordingly
                let wait_deadline = resp.headers()
                    .get(RETRY_AFTER)
                    .and_then(|hval| hval.to_str().ok()) // header values aren't always ASCII strings
                    .and_then(get_http_header_deadline)
                    .unwrap_or(Instant::now() + Duration::from_secs(20)); // 20s default

                future::Either::B(
                    Delay::new(wait_deadline)
                        .map_err(|e| Error::new(ErrorKind::Other, e))
                )
            })
    };

    item_rx
        .map_err(|_| Error::from(ErrorKind::BrokenPipe))
        .map(process_item)
        .buffer_unordered(cfg.max_inflight) // Limit parallelism
        .for_each(|_| Ok(()))
}

/// Posts back feedback to the queue server about succeeded and perma-failed
/// API requests.
fn report_feedback(
    cfg: RunCfg,
    client: Client,
    rx: mpsc::Receiver<QueueItemFeedback>,
) -> impl Future<Item = (), Error = Error> {
    rx
        .map_err(|_| Error::from(ErrorKind::BrokenPipe))
        .and_then(move |feedback| {
            client.post(&cfg.remote_url)
                .header(AUTHORIZATION, format!("Bearer {}", cfg.auth_token))
                .json(&QueueItemFeedbackRequest {
                    items: vec![feedback]
                })
                .send()
                .then(|_| Ok(()))
        })
        .for_each(|_| Ok(()))
}

/// Get the deadline specified within the given header value string
/// that is either a duration in seconds or a deadline given by in
/// IMF-fixdate.
fn get_http_header_deadline(val: &str) -> Option<Instant> {
    val.trim().parse().ok()
        .map(Duration::from_secs)
        .or_else(|| {
            parse_http_date(val).ok()
                .and_then(|time| time.duration_since(SystemTime::now()).ok())
        })
        .map(|dur| Instant::now() + dur)
}
