use pbr::{MultiBar, Pipe, ProgressBar, Units};
use std::io::Stdout;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

lazy_static! {
    static ref TOTALS: Mutex<Vec<u64>> = Mutex::new(vec![]);
    static ref PBRS: Mutex<Vec<ProgressBar<Pipe>>> = Mutex::new(vec![]);
}

pub fn start_pbr(file_name: String, lengths: Vec<u64>) {
    let mut mb = MultiBar::new();
    mb.println(&format!("Downloading: {}", file_name));

    let total_length = lengths.iter().fold(0, |a, &b| a + b);
    build_global_bar(&mut mb, total_length);
    mb.println("");

    for length in lengths {
        build_child_bar(&mut mb, length);
    }

    thread::spawn(move || mb.listen());
}

pub fn setting_up_bar(bar: usize) {
    let mut pbrs = PBRS
        .lock()
        .expect("Failed to aquire PBRS lock, lock poisoned!");
    pbrs[bar + 1].message("Starting... ");
    pbrs[bar + 1].tick();
}

pub fn start_bar(bar: usize) {
    let mut pbrs = PBRS
        .lock()
        .expect("Failed to aquire PBRS lock, lock poisoned!");
    pbrs[bar + 1].message("");
    pbrs[bar + 1].show_message = false;
    pbrs[bar + 1].tick();
}

pub fn update_bar(bar: usize, progress: u64) {
    let mut pbrs = PBRS
        .lock()
        .expect("Failed to aquire PBRS lock, lock poisoned!");
    let mut totals = TOTALS
        .lock()
        .expect("Failed to aquire TOTALS lock, lock poisoned!");

    pbrs[bar + 1].set(progress);
    totals[bar] = progress;

    let total_progress = totals.iter().fold(0, |a, &b| a + b);
    pbrs[0].set(total_progress);
}

pub fn success_global_bar() {
    finish_bar_with_message(0, "Download Complete!");
}

pub fn success_bar(bar: usize) {
    finish_bar_with_message(bar + 1, "Download Complete!");
}

pub fn fail_bar(bar: usize) {
    finish_bar_with_message(bar + 1, "Download Failed!");
}

fn finish_bar_with_message(act_bar: usize, message: &str) {
    PBRS.lock()
        .expect("Failed to aquire PBRS lock, lock poisoned!")[act_bar]
        .finish_print(message);
}

fn build_global_bar(mb: &mut MultiBar<Stdout>, size: u64) {
    build_bar(mb, size, None);
}

fn build_child_bar(mb: &mut MultiBar<Stdout>, size: u64) {
    build_bar(mb, size, Some("Pending... ".to_string()));

    let mut totals = TOTALS
        .lock()
        .expect("Failed to aquire TOTALS lock, lock poisoned!");
    totals.push(0);
}

fn build_bar(mb: &mut MultiBar<Stdout>, size: u64, message: Option<String>) {
    let mut pbrs = PBRS
        .lock()
        .expect("Failed to aquire PBRS lock, lock poisoned!");
    let mut pb = mb.create_bar(size);
    pb.set_max_refresh_rate(Some(Duration::from_millis(200)));
    pb.tick_format("▏▎▍▌▋▊▉██▉▊▋▌▍▎▏");
    pb.set_units(Units::Bytes);

    if let Some(msg) = message {
        pb.show_message = true;
        pb.message(&msg);
    } else {
        pb.show_message = false;
    }

    pb.tick();
    pbrs.push(pb);
}
