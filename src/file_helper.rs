use bytes::{BytesMut, BufMut, BigEndian};
use reqwest::{Error, Response};
use reqwest::header::{ContentRange, ContentRangeSpec};
use std::fs::{self, rename, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::ops::Deref;
use std::sync::Mutex;
use ui_helper;

lazy_static! {
    static ref FLOCK: Mutex<u8> = Mutex::new(0u8);
}

pub const CHUNK_SIZE_USIZE: usize = 128 * 1024;
pub const CHUNK_SIZE_U64: u64 = 128 * 1024;

pub fn create_file(path: String, bytes: u64) -> u64 {
    let (chunk_count, chunk_space) = calculate_chunk_count_and_space(bytes);
    let tmp_name = tmp_file_name(path);

    let footer_space = chunk_space + 8;
    let footer_space_u64 = footer_space as u64;
    let existing_file_length =
        fs::metadata(tmp_name.clone()).map(|metadata| metadata.len()).unwrap_or(0);

    if existing_file_length != footer_space_u64 + bytes {
        let mut buf = BytesMut::with_capacity(footer_space);
        for _ in 0..chunk_space {
            buf.put_u8(0);
        }
        buf.put_u64::<BigEndian>(chunk_count);

        let mut file = File::create(tmp_name).unwrap();
        file.set_len(footer_space_u64 + bytes).unwrap();
        file.seek(SeekFrom::End(-(footer_space_u64 as i64))).unwrap();
        file.write_all(&buf[..]).unwrap();
    }
    footer_space_u64
}

pub fn remove_footer_and_save(path: String, bytes: u64) {
    let tmp_path = tmp_file_name(path.clone());
    let file = OpenOptions::new().write(true).open(tmp_path.clone()).unwrap();
    file.set_len(bytes).unwrap();
    rename(tmp_path, path).unwrap();
}

pub fn save_response(path: String,
                     mut res: Response,
                     footer_space: u64,
                     child_id: usize,
                     prefilled: u64)
                     -> Result<u64, Error> {
    let first_byte = match *res.headers().get::<ContentRange>().unwrap().deref() {
        ContentRangeSpec::Bytes { range, instance_length: _ } => range.unwrap().0,
        _ => panic!("Response header of incorrect form!"),
    };

    let file_name = tmp_file_name(path.clone());
    let mut file = OpenOptions::new().write(true).read(true).open(file_name).unwrap();

    file.seek(SeekFrom::Start(first_byte)).unwrap();
    let mut buf = [0; CHUNK_SIZE_USIZE];
    let mut written = prefilled;

    while let Ok(len) = res.read(&mut buf) {
        if len == 0 {
            return Ok(written);
        }
        file.write_all(&buf[..len]).unwrap();
        let last_byte_update = (written + first_byte) / CHUNK_SIZE_U64;
        written += len as u64;
        let new_byte_update = (written + first_byte) / CHUNK_SIZE_U64;
        set_written_chunks(path.clone(),
                           footer_space,
                           (last_byte_update, new_byte_update));
        ui_helper::update_bar(child_id, written);
    }

    return Ok(0u64);
}

pub fn get_first_empty_chunk(path: String, footer_space: u64, byte_range: (u64, u64)) -> u64 {
    let _guard = FLOCK.lock().expect("Failed to aquire lock, lock poisoned!");
    let mut file = OpenOptions::new().read(true).open(tmp_file_name(path)).unwrap();
    // let (first_byte, last_byte) = byte_range;
    let first_chunk = byte_range.0 / CHUNK_SIZE_U64;
    let last_chunk = byte_range.1 / CHUNK_SIZE_U64;
    let first_byte = get_chunk_status_offset(footer_space as i64, first_chunk as i64);
    let last_byte = get_chunk_status_offset(footer_space as i64, last_chunk as i64);

    file.seek(SeekFrom::End(first_byte)).unwrap();
    let mut buf = [0; 1];
    let mut chunk_num = first_chunk;
    for byte_num in first_byte..(last_byte + 1) {
        file.read(&mut buf).unwrap();

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

fn set_written_chunks(path: String, footer_space: u64, chunk_range: (u64, u64)) {
    let (first_chunk, last_chunk) = chunk_range;
    if last_chunk <= first_chunk {
        return;
    }

    let _guard = FLOCK.lock().expect("Failed to aquire lock, lock poisoned!");
    let mut file = OpenOptions::new().write(true).read(true).open(tmp_file_name(path)).unwrap();

    let first_byte = get_chunk_status_offset(footer_space as i64, first_chunk as i64);
    let last_byte = get_chunk_status_offset(footer_space as i64, last_chunk as i64);
    let mut buf = [0; 1];
    for byte_num in first_byte..(last_byte + 1) {
        file.seek(SeekFrom::End(byte_num)).unwrap();
        file.read(&mut buf).unwrap();

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

        let mut byte = buf[0];

        for bit_offset in start_offset..finish_offset {
            byte = byte | (1 << (7 - bit_offset));
        }

        file.seek(SeekFrom::End(byte_num)).unwrap();
        file.write(&[byte]).unwrap();
    }
}

fn get_chunk_status_offset(footer_space: i64, chunk: i64) -> i64 {
    -footer_space + (chunk / 8) + 8
}

fn tmp_file_name(path: String) -> String {
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
