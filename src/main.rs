#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#![cfg_attr(feature="clippy", deny(clippy_pedantic))]
#![cfg_attr(feature="clippy", allow(missing_docs_in_private_items))]

#![deny(missing_debug_implementations, missing_copy_implementations,
    trivial_casts, trivial_numeric_casts, unsafe_code,
    unused_import_braces, unused_qualifications)]

#[macro_use]
extern crate clap;
#[macro_use]
extern crate slog;
extern crate slog_term;
extern crate reqwest;
extern crate crypto;
extern crate url;
extern crate uuid;
extern crate rustc_serialize as serialize;

mod auth_helper;
use clap::App;
use reqwest::{Client, Url};
use reqwest::header::{AcceptRanges, ContentLength, RangeUnit};
use std::ops::Deref;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    let yaml = load_yaml!("cli.yml");
    let m = App::from_yaml(yaml)
        .version(VERSION)
        .get_matches();

    #[cfg_attr(feature="clippy", allow(option_unwrap_used))]
    let raw_uri = m.value_of("uri").unwrap(); // Unwrap is safe - required by clap
    let url = match Url::parse(raw_uri) {
        Ok(uri) => uri,
        Err(e) => panic!("Couldn't parse URI: {}", e),
    };

    let da = auth_helper::AuthenticationRequest::new(raw_uri.to_owned(),
                                                     url.username().to_owned(),
                                                     url.password()
                                                         .map(|password| password.to_owned()),
                                                     Some("HEAD".to_owned()));

    let cli = Client::new().unwrap();
    let resp = match da.authenticate() {
        Ok(Some(headers)) => cli.head(url).headers(headers).send().unwrap(),
        Ok(None) => cli.head(url).send().unwrap(),
        Err(e) => panic!(e), // this is genuine error, authentication was not attempted
    };

    let headers = resp.headers();
    if !headers.get::<AcceptRanges>().map_or(false, |range_header| {
        range_header.deref().contains(&RangeUnit::Bytes)
    }) {
        panic!("Requested resource does not allow Range requests!")
    }

    let content_length = headers.get::<ContentLength>()
        .map_or(0, |length_header| *length_header.deref());

    if content_length < 1024 {
        panic!("Content too small");
    }
}
