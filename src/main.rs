#![recursion_limit="128"]

#[macro_use] extern crate clap;
extern crate futures;
extern crate httpdate;
extern crate reqwest;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate tokio;

mod poll;
mod types;

use clap::{Arg, ArgMatches};
use std::time::Duration;

const AUTH_TOKEN_ARG: &str = "AUTH_TOKEN";
const DEFAULT_429_DELAY: &str = "DEFAULT_DELAY";
const MAX_INFLIGHT_ARG: &str = "MAX_PARALLELISM";
const POLL_INTERVAL: &str = "POLL_INTERVAL";
const URL_ARG: &str = "URL";

fn main() {
    let matches: ArgMatches = app_from_crate!()
        .arg(
            Arg::with_name(AUTH_TOKEN_ARG)
                .help("Access token to be provided to the queue endpoint.")
                .long("token")
                .short("t")
                .required(true)
                .takes_value(true)
        )
        .arg(
            Arg::with_name(DEFAULT_429_DELAY)
                .help("The time to wait (in seconds) after receiving a 429 status code, if no Retry-After header was present.")
                .long("delay")
                .default_value("20")
                .takes_value(true)
        )
        .arg(
            Arg::with_name(MAX_INFLIGHT_ARG)
                .help("Maximum number of requests to be in flight at the same time.")
                .long("parallel")
                .short("p")
                .default_value("1")
                .takes_value(true)
        )
        .arg(
            Arg::with_name(POLL_INTERVAL)
                .help("The interval (in seconds) to poll for new queue items at.")
                .short("i")
                .long("poll-interval")
                .default_value("10")
                .takes_value(true)
        )
        .arg(
            Arg::with_name(URL_ARG)
                .help("The URL to query for queue items.")
                .long("url")
                .short("u")
                .required(true)
                .takes_value(true)
        )
        .get_matches();

    let cfg = poll::RunCfg {
        auth_token: matches.value_of(AUTH_TOKEN_ARG).unwrap().to_owned(),
        default_delay: matches.value_of(DEFAULT_429_DELAY)
            .unwrap()
            .parse()
            .map(Duration::from_secs)
            .expect("Invalid delay number format"),
        max_inflight: matches.value_of(MAX_INFLIGHT_ARG)
            .unwrap()
            .parse()
            .expect("Invalid parallelism number format"),
        poll_interval: matches.value_of(POLL_INTERVAL)
            .unwrap()
            .parse()
            .map(Duration::from_secs)
            .expect("Invalid poll interval number format"),
        remote_url: matches.value_of(URL_ARG).unwrap().to_owned(),
    };
    poll::run(cfg);
}
