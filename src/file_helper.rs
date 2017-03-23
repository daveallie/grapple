use bytes::{BytesMut, BufMut, BigEndian};
use reqwest::{Error, Response};
use reqwest::header::{ContentRange, ContentRangeSpec};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::ops::Deref;
use std::sync::{Arc, Mutex};

const CHUNK_SIZE_USIZE: usize = 128 * 1024;
const CHUNK_SIZE_U64: u64 = 128 * 1024;

pub fn create_file(path: String, bytes: u64) -> u64 {
    let (chunk_count, chunk_space) = calculate_chunk_count_and_space(bytes);

    let header_space = 8 + chunk_space;
    let mut buf = BytesMut::with_capacity(header_space);
    buf.put_u64::<BigEndian>(chunk_count);
    for _ in 0..chunk_space {
        buf.put_u8(0);
    }

    let header_space = header_space as u64;

    let mut file = File::create(tmp_file_name(path)).unwrap();
    file.set_len(header_space + bytes).unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();
    file.write_all(&buf[..]).unwrap();
    header_space
}

pub fn save_response(path: String,
                     mut res: Response,
                     header_space: u64,
                     lock: Arc<Mutex<u8>>)
                     -> Result<u64, Error> {
    let first_byte = match *res.headers().get::<ContentRange>().unwrap().deref() {
        ContentRangeSpec::Bytes { range, instance_length: _ } => range.unwrap().0,
        _ => panic!("Response header of incorrect form!"),
    };

    let file_name = tmp_file_name(path);
    let mut file = OpenOptions::new().write(true).read(true).open(file_name).unwrap();

    file.seek(SeekFrom::Start(header_space + first_byte)).unwrap();
    let mut buf = [0; CHUNK_SIZE_USIZE];
    let mut written = 0;

    while let Ok(len) = res.read(&mut buf) {
        if len == 0 {
            set_written_chunks(file,
                               (first_byte / CHUNK_SIZE_U64,
                                (written + first_byte) / CHUNK_SIZE_U64),
                               lock);
            return Ok(written);
        }
        file.write_all(&buf[..len]).unwrap();
        written += len as u64;
    }

    return Ok(0u64);
}

fn set_written_chunks(mut file: File, chunk_range: (u64, u64), lock: Arc<Mutex<u8>>) {
    let (first_chunk, last_chunk) = chunk_range;
    if last_chunk <= first_chunk {
        return;
    }

    let _guard = lock.lock().unwrap_or_else(|_| {
        panic!("Failed to aquire lock, lock poisoned!");
    });

    let first_byte = get_chunk_status_offset(first_chunk);
    let last_byte = get_chunk_status_offset(last_chunk);
    let mut buf = [0; 1];
    for byte_num in first_byte..(last_byte + 1) {
        file.seek(SeekFrom::Start(byte_num)).unwrap();
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

        file.seek(SeekFrom::Start(byte_num)).unwrap();
        file.write(&[byte]).unwrap();
    }
}

fn get_chunk_status_offset(chunk: u64) -> u64 {
    8 + (chunk / 8)
}

fn tmp_file_name(path: String) -> String {
    format!("{}.tmp", path)
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
