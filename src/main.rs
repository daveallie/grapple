#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#![cfg_attr(feature="clippy", deny(clippy_pedantic))]
#![cfg_attr(feature="clippy", allow(missing_docs_in_private_items))]

#![deny(missing_debug_implementations, missing_copy_implementations,
    trivial_casts, trivial_numeric_casts, unsafe_code,
    unused_import_braces, unused_qualifications)]

#[macro_use]
extern crate clap;
extern crate reqwest;
extern crate crypto;
extern crate url;
extern crate uuid;
extern crate rustc_serialize as serialize;
extern crate bytes;
extern crate pbr;
#[macro_use]
extern crate lazy_static;

mod auth_helper;
mod file_helper;
mod request_helper;
mod ui_helper;

use clap::App;
use reqwest::Url;
use reqwest::header::{AcceptRanges, ContentLength, RangeUnit};
use std::{process, thread};
use std::ops::Deref;
use std::path::Path;

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

    let file_name = request_helper::get_last_url_segment(url.clone());

    if Path::new(&file_name).exists() {
        println!("{} already exists, please remove it and try again.",
                 file_name);
        process::exit(1);
    }

    let thread_count = m.value_of("thread_count")
        .map(|tc| tc.parse::<u64>().expect("Failed to parse thread count."))
        .unwrap_or(10);

    if thread_count > 20 {
        panic!("Thread count too high, please select between 2 and 20 threads.");
    } else if thread_count < 2 {
        panic!("Thread count too low, please select between 2 and 20 threads.");
    }

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

    let part_length = (content_length / thread_count) / file_helper::CHUNK_SIZE_U64 *
                      file_helper::CHUNK_SIZE_U64;

    let mut sections: Vec<(u64, u64)> = vec![];
    let mut lengths: Vec<u64> = vec![];
    for section in 0..(thread_count - 1) {
        sections.push((section * part_length, (section + 1) * part_length - 1));
        lengths.push(part_length);
    }
    sections.push(((thread_count - 1) * part_length, content_length));
    lengths.push(content_length - (thread_count - 1) * part_length);

    ui_helper::start_pbr(file_name.clone(), lengths);

    let footer_space = file_helper::create_file(file_name.clone(), content_length);
    let mut children = vec![];
    for child_id in 0..(thread_count as usize) {
        let section = sections[child_id];
        let url_clone = url.clone();
        let file_name_clone = file_name.clone();
        let child = thread::spawn(move || {
            ui_helper::setting_up_bar(child_id);
            let start =
                file_helper::get_first_empty_chunk(file_name_clone.clone(), footer_space, section);
            if start <= section.1 {
                let prefilled = start - section.0;
                let section = (start, section.1);
                let range_req = request_helper::get_range_request(url_clone.clone(), section);
                ui_helper::start_bar(child_id);
                let written = file_helper::save_response(file_name_clone.clone(),
                                                         range_req,
                                                         footer_space,
                                                         child_id,
                                                         prefilled)
                    .unwrap();
                if written > 0 {
                    ui_helper::success_bar(child_id);
                } else {
                    ui_helper::fail_bar(child_id);
                }
            } else {
                ui_helper::success_bar(child_id);
            }
        });
        children.push(child);
    }
    for child in children {
        let _ = child.join();
    }
    file_helper::remove_footer_and_save(file_name, content_length);
}
