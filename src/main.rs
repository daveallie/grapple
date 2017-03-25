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
extern crate bytes;

mod auth_helper;
mod file_helper;
mod request_helper;

use clap::App;
use reqwest::Url;
use reqwest::header::{AcceptRanges, ContentLength, RangeUnit};
use std::ops::Deref;
use std::sync::{Arc, Mutex};

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

    let res = request_helper::head_request(url.clone());
    let headers = res.headers();
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

    let part_length = content_length / 10;

    let mut sections: Vec<(u64, u64)> = vec![];
    for section in 0..9 {
        sections.push((section * part_length, (section + 1) * part_length - 1));
    }
    sections.push((9 * part_length, content_length));
    println!("{:?}", sections);

    let chunk_status_write_lock: Arc<Mutex<u8>> = Arc::new(Mutex::new(0u8));

    let footer_space = file_helper::create_file("test_file".to_owned(), content_length);
    for section in sections {
        let range_req = request_helper::get_range_request(url.clone(), section);
        let written = file_helper::save_response("test_file".to_owned(),
                                                 range_req,
                                                 footer_space,
                                                 chunk_status_write_lock.clone());
        println!("{}", written.unwrap());
    }
    file_helper::remove_footer_and_save("test_file".to_owned(), content_length);
}
