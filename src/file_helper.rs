use reqwest::header::CONTENT_RANGE;
use reqwest::{Error, Response};
use regex::Regex;
use std::fs::{self, rename, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::mem::transmute;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use ui_helper;

lazy_static! {
    static ref FLOCK: Mutex<()> = Mutex::new(());
}

pub const CHUNK_SIZE_USIZE: usize = 128 * 1024;
pub const CHUNK_SIZE_U64: u64 = 128 * 1024;

pub fn create_file(path: &str, bytes: u64) -> u64 {
    let (chunk_count, chunk_space) = calculate_chunk_count_and_space(bytes);
    let tmp_name = tmp_file_name(path);

    let footer_space = chunk_space + 8;
    let footer_space_u64 = footer_space as u64;
    let existing_file_length = fs::metadata(tmp_name.clone())
        .map(|metadata| metadata.len())
        .unwrap_or(0);

    if existing_file_length != footer_space_u64 + bytes {
        let mut buf: Vec<u8> = vec![0_u8; chunk_space];

        #[allow(unsafe_code)]
        let chunk_count_bytes: [u8; 8] = unsafe { transmute(chunk_count.to_be()) };
        buf.extend_from_slice(&chunk_count_bytes);

        let mut file = File::create(tmp_name).unwrap();
        file.set_len(footer_space_u64 + bytes).unwrap();
        file.seek(SeekFrom::End(-(footer_space_u64 as i64)))
            .unwrap();
        file.write_all(&buf[..]).unwrap();
    }
    footer_space_u64
}

pub fn remove_footer_and_save(path: &str, bytes: u64) {
    let tmp_path = tmp_file_name(path);
    let file = OpenOptions::new()
        .write(true)
        .open(tmp_path.clone())
        .unwrap();
    file.set_len(bytes).unwrap();
    rename(tmp_path, path).unwrap();
}

pub fn save_response(
    path: &str,
    mut res: Response,
    footer_space: u64,
    child_id: usize,
    prefilled: u64,
    thread_bandwidth: Option<u32>,
) -> Result<u64, Error> {
    let content_range: String = (*res.headers().get(CONTENT_RANGE).unwrap().to_str().unwrap()).to_string();
    let range_regex = Regex::new(r"(?<unit>^[a-zA-Z][\w]*)\s+(?<rangeStart>\d+)\s?-\s?(?<rangeEnd>\d+)?\s?/\s?(?<size>\d+|\*)?").unwrap();
    let first_byte = match range_regex.captures(&content_range) {
        Some(matches) => {
            matches["rangeStart"].parse::<u64>().unwrap()
        },
        None => panic!("Invalid Content-Range header in Response")
    };

    let file_name = tmp_file_name(path);
    let mut file = OpenOptions::new()
        .write(true)
        .read(true)
        .open(file_name)
        .unwrap();

    file.seek(SeekFrom::Start(first_byte)).unwrap();
    let mut buf = [0; CHUNK_SIZE_USIZE];
    let mut written = 0;
    let mut last_bw_sync = Instant::now();
    let mut bytes_since_bw_sync: f64 = 0.0;
    let bandwidth = thread_bandwidth.map(|bw| f64::from(bw) * 1024_f64);
    let bytes_between_bw_sync = bandwidth.unwrap_or(0.0) * 0.1;

    while let Ok(len) = res.read(&mut buf) {
        if len == 0 {
            return Ok(written + prefilled);
        }
        file.write_all(&buf[..len]).unwrap();
        let last_working_chunk = (written + first_byte) / CHUNK_SIZE_U64;
        written += len as u64;
        let current_working_chunk = (written + first_byte) / CHUNK_SIZE_U64;
        set_written_chunks(
            path,
            footer_space,
            (last_working_chunk, current_working_chunk),
        );
        ui_helper::update_bar(child_id, written + prefilled);

        if let Some(bw) = bandwidth {
            bytes_since_bw_sync += len as f64;

            if bytes_since_bw_sync >= bytes_between_bw_sync {
                let seconds_wait = len as f64 / bw;
                let wait_time = Duration::new(
                    seconds_wait.trunc() as u64,
                    (seconds_wait * 1_000_000_000_f64) as u32,
                );
                let time_passed = Instant::now() - last_bw_sync;

                if wait_time.gt(&time_passed) {
                    thread::sleep(wait_time - time_passed);
                }

                last_bw_sync = Instant::now();
            }
        }
    }

    Ok(0u64)
}

pub fn get_first_empty_chunk(path: &str, footer_space: u64, byte_range: (u64, u64)) -> u64 {
    let _guard = FLOCK
        .lock()
        .expect("Failed to acquire lock, lock poisoned!");
    let mut file = OpenOptions::new()
        .read(true)
        .open(tmp_file_name(path))
        .unwrap();
    let first_chunk = byte_range.0 / CHUNK_SIZE_U64;
    let last_chunk = byte_range.1 / CHUNK_SIZE_U64;
    let first_byte = get_chunk_status_offset(footer_space as i64, first_chunk as i64);
    let last_byte = get_chunk_status_offset(footer_space as i64, last_chunk as i64);

    file.seek(SeekFrom::End(first_byte)).unwrap();
    let mut buf = [0; 1];
    let mut chunk_num = first_chunk;
    for byte_num in first_byte..=last_byte {
        file.read_exact(&mut buf).unwrap();

        let start_offset = if byte_num == first_byte {
            first_chunk % 8
        } else {
            0
        };

        let finish_offset = if byte_num == last_byte {
            last_chunk % 8 + 1
        } else {
            8
        };

        let byte = buf[0];

        for bit_offset in start_offset..finish_offset {
            if byte & (1 << (7 - bit_offset)) == 0 {
                return chunk_num * CHUNK_SIZE_U64;
            }
            chunk_num += 1;
        }
    }
    chunk_num * CHUNK_SIZE_U64
}

fn set_written_chunks(path: &str, footer_space: u64, working_chunk_from_to: (u64, u64)) {
    let (last_working_chunk, current_working_chunk) = working_chunk_from_to;
    if current_working_chunk <= last_working_chunk {
        return;
    }

    let current_complete_chunk = current_working_chunk - 1;

    let _guard = FLOCK
        .lock()
        .expect("Failed to acquire lock, lock poisoned!");
    let mut file = OpenOptions::new()
        .write(true)
        .read(true)
        .open(tmp_file_name(path))
        .unwrap();

    let first_byte = get_chunk_status_offset(footer_space as i64, last_working_chunk as i64);
    let last_byte = get_chunk_status_offset(footer_space as i64, current_complete_chunk as i64);
    let mut buf = [0; 1];
    for byte_num in first_byte..=last_byte {
        file.seek(SeekFrom::End(byte_num)).unwrap();
        file.read_exact(&mut buf).unwrap();

        let start_offset = if byte_num == first_byte {
            last_working_chunk % 8
        } else {
            0
        };

        let finish_offset = if byte_num == last_byte {
            current_complete_chunk % 8 + 1
        } else {
            8
        };

        let mut byte = buf[0];

        for bit_offset in start_offset..finish_offset {
            byte |= 1 << (7 - bit_offset);
        }

        file.seek(SeekFrom::End(byte_num)).unwrap();
        file.write_all(&[byte]).unwrap();
    }
}

fn get_chunk_status_offset(footer_space: i64, chunk: i64) -> i64 {
    -footer_space + (chunk / 8)
}

fn tmp_file_name(path: &str) -> String {
    format!("{}.grapplepartial", path)
}

fn calculate_chunk_count_and_space(bytes: u64) -> (u64, usize) {
    let mut num_chunks = bytes / CHUNK_SIZE_U64;
    if bytes % CHUNK_SIZE_U64 > 0 {
        num_chunks += 1;
    }
    let mut chunk_space = num_chunks / 8;
    if num_chunks % 8 > 0 {
        chunk_space += 1;
    }

    (num_chunks, chunk_space as usize)
}
