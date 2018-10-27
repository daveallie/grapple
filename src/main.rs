#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications
)]

#[macro_use]
extern crate clap;
extern crate base64;
extern crate md5;
extern crate indicatif;
extern crate reqwest;
extern crate url;
extern crate uuid;
#[macro_use]
extern crate lazy_static;

mod auth_helper;
mod file_helper;
mod request_helper;
mod ui_helper;

use clap::App;
use reqwest::header::{AcceptRanges, ContentLength, RangeUnit};
use reqwest::Url;
use std::collections::HashSet;
use std::ops::Deref;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use std::{process, thread};

const VERSION: &str = env!("CARGO_PKG_VERSION");

lazy_static! {
    static ref CURRENTLY_RUNNING_THREAD_IDS: Mutex<HashSet<usize>> = Mutex::new(HashSet::new());
    static ref HAS_FAILED: AtomicBool = AtomicBool::new(false);
}

fn main() {
    let yaml = load_yaml!("cli.yml");
    let m = App::from_yaml(yaml).version(VERSION).get_matches();

    #[cfg_attr(feature = "clippy", allow(option_unwrap_used))]
    let raw_uri = m.value_of("uri").unwrap(); // Unwrap is safe - required by clap
    let mut url = match Url::parse(raw_uri) {
        Ok(uri) => uri,
        Err(e) => panic!("Couldn't parse URI: {}", e),
    };

    let file_name = request_helper::get_last_url_segment_decoded(&url);

    if Path::new(&file_name).exists() {
        println!(
            "{} already exists, please remove it and try again.",
            file_name
        );
        process::exit(1);
    }

    let thread_count = m
        .value_of("thread_count")
        .map(|tc| tc.parse::<usize>().expect("Failed to parse thread count."))
        .unwrap_or(10);
    let part_count = m
        .value_of("part_count")
        .map(|tc| tc.parse::<usize>().expect("Failed to parse part count."))
        .unwrap_or(thread_count);
    let part_count_u64 = part_count as u64;

    if part_count < thread_count {
        panic!("Part count too low, must be at least the thread count.");
    }

    if thread_count > 20 {
        panic!("Thread count too high, please select between 2 and 20 threads.");
    } else if thread_count < 2 {
        panic!("Thread count too low, please select between 2 and 20 threads.");
    }

    let username = m
        .value_of("username")
        .map(|u| u.parse::<String>().expect("Failed to parse username."));
    let password = m
        .value_of("password")
        .map(|p| p.parse::<String>().expect("Failed to parse password."));

    request_helper::override_username_password(&mut url, username, password);

    let thread_bandwidth = m.value_of("thread_bandwidth").map(|bw| {
        bw.parse::<u32>()
            .expect("Failed to parse thread bandwidth.")
    });

    let url = url;
    let res = request_helper::head_request(url.clone());
    let headers = res.headers();
    let has_range_header = headers.get::<AcceptRanges>().map_or(false, |range_header| {
        range_header.deref().contains(&RangeUnit::Bytes)
    });
    if !has_range_header {
        panic!("Requested resource does not allow Range requests!")
    }

    let content_length = headers
        .get::<ContentLength>()
        .map_or(0, |length_header| *length_header.deref());

    if content_length < 1024 {
        panic!("Content too small");
    }

    let part_length = (content_length / part_count_u64) / file_helper::CHUNK_SIZE_U64
        * file_helper::CHUNK_SIZE_U64;

    let mut sections: Vec<(u64, u64)> = vec![];
    let mut lengths: Vec<u64> = vec![];
    for section in 0..(part_count_u64 - 1) {
        sections.push((section * part_length, (section + 1) * part_length - 1));
        lengths.push(part_length);
    }
    sections.push(((part_count_u64 - 1) * part_length, content_length - 1));
    lengths.push(content_length - (part_count_u64 - 1) * part_length);

    ui_helper::start_all_pb(&file_name, thread_count, lengths);

    let footer_space = file_helper::create_file(&file_name, content_length);
    let mut children = vec![];
    for (child_id, section) in sections.into_iter().enumerate() {
        let url_clone = url.clone();
        let file_name_clone = file_name.clone();
        let mut thread_id = 0;
        loop {
            {
                let mut currently_running_ids = CURRENTLY_RUNNING_THREAD_IDS
                    .lock()
                    .expect("Failed to acquire lock, lock poisoned!");
                if currently_running_ids.len() < thread_count {
                    for t_id in 1..=thread_count {
                        if !currently_running_ids.iter().any(|x| *x == t_id) {
                            currently_running_ids.insert(t_id);
                            thread_id = t_id;
                            break;
                        }
                    }
                    break;
                }
            }
            thread::sleep(Duration::new(1, 0));
        }
        let child = thread::spawn(move || {
            let start = file_helper::get_first_empty_chunk(&file_name_clone, footer_space, section);
            if start <= section.1 {
                ui_helper::setting_up_bar(thread_id, child_id, section.1 - section.0 + 1);
                let prefilled = start - section.0;
                let section = (start, section.1);
                let range_req = request_helper::get_range_request(url_clone.clone(), section);
                let written = file_helper::save_response(
                    &file_name_clone,
                    range_req,
                    footer_space,
                    child_id,
                    thread_id,
                    prefilled,
                    thread_bandwidth,
                )
                .unwrap();
                if written > 0 {
                    ui_helper::success_bar(thread_id);
                } else {
                    ui_helper::fail_bar(thread_id);
                    HAS_FAILED.store(true, Ordering::Release);
                }
            } else {
                ui_helper::adjust_totals(child_id, section.1 - section.0 + 1);
            }

            let mut currently_running_ids = CURRENTLY_RUNNING_THREAD_IDS
                .lock()
                .expect("Failed to acquire lock, lock poisoned!");

            currently_running_ids.remove(&thread_id);
        });
        children.push(child);
    }
    for child in children {
        let _ = child.join();
    }

    if HAS_FAILED.load(Ordering::Acquire) {
        println!("Some parts failed to download, please rerun.");
        process::exit(1);
    } else {
        file_helper::remove_footer_and_save(&file_name, content_length);
    }
}
