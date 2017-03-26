use pbr::{ProgressBar, Pipe, MultiBar, Units};
use std::io::Stdout;
use std::sync::Mutex;
use std::thread;

lazy_static! {
    static ref PBRS: Mutex<Vec<ProgressBar<Pipe>>> = Mutex::new(vec![]);
}


pub fn start_pbr(file_name: String, lengths: Vec<u64>) {
    let mut mb = MultiBar::new();
    mb.println(&format!("Downloading: {}", file_name));
    mb.println("");

    for length in lengths {
        build_bar(&mut mb, length);
    }

    thread::spawn(move || mb.listen());
}

pub fn update_bar(bar: usize, progress: u64) {
    PBRS.lock().expect("Failed to aquire PBRS lock, lock poisoned!")[bar].set(progress);
}

pub fn success_bar(bar: usize) {
    finish_bar_with_message(bar, "Download Complete!");
}

pub fn fail_bar(bar: usize) {
    finish_bar_with_message(bar, "Download Failed! Retrying.");
}

fn finish_bar_with_message(bar: usize, message: &str) {
    PBRS.lock().expect("Failed to aquire PBRS lock, lock poisoned!")[bar].finish_print(message);
}

fn build_bar(mb: &mut MultiBar<Stdout>, size: u64) {
    let mut pbrs = PBRS.lock().expect("Failed to aquire PBRS lock, lock poisoned!");
    let mut pb = mb.create_bar(size);
    pb.tick_format("▏▎▍▌▋▊▉██▉▊▋▌▍▎▏");
    pb.set_units(Units::Bytes);
    pb.tick();
    pbrs.push(pb);
}