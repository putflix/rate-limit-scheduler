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

const AUTH_TOKEN_ARG: &str = "AUTH_TOKEN";
const MAX_INFLIGHT_ARG: &str = "MAX_PARALLELISM";
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
            Arg::with_name(MAX_INFLIGHT_ARG)
                .help("Maximum number of requests to be in flight at the same time.")
                .long("parallel")
                .short("p")
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
        max_inflight: matches.value_of(MAX_INFLIGHT_ARG).unwrap()
            .parse().expect("Invalid parallelism number format"),
        remote_url: matches.value_of(URL_ARG).unwrap().to_owned(),
    };
    poll::run(cfg);
}
