use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::Mutex;
use std::thread;

lazy_static! {
    static ref TOTALS: Mutex<Vec<u64>> = Mutex::new(vec![]);
    static ref PBRS: Mutex<Vec<ProgressBar>> = Mutex::new(vec![]);
}

pub fn start_all_pb(file_name: &str, thread_count: usize, lengths: Vec<u64>) {
    let mut mb = MultiProgress::new();
    mb.set_move_cursor(true);

    println!("{}", file_name);

    let total_length = lengths.iter().sum();
    let master_style = ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {percent}% [{wide_bar:.green}] {bytes}/{total_bytes}")
        .progress_chars("█▉▊▋▌▍▎▏  ");
    build_bar(&mut mb, master_style, total_length);

    {
        let mut totals = TOTALS
            .lock()
            .expect("Failed to acquire TOTALS lock, lock poisoned!");
        for _i in 0..lengths.len() {
            totals.push(0);
        }
    }

    for &length in lengths.iter().take(thread_count) {
        let child_style = ProgressStyle::default_bar()
            .template("{msg} [{wide_bar:.yellow}] {bytes}/{total_bytes}")
            .progress_chars("█▉▊▋▌▍▎▏  ");

        build_bar(&mut mb, child_style.clone(), length);
    }

    thread::spawn(move || mb.join_and_clear().unwrap());
}

pub fn setting_up_bar(thread_id: usize, part_id: usize, size: u64) {
    let pbrs = PBRS
        .lock()
        .expect("Failed to acquire PBRS lock, lock poisoned!");

    pbrs[thread_id].set_length(size);
    pbrs[thread_id].set_position(0);
    pbrs[thread_id].set_message(&format!("Part {}", part_id + 1));
    pbrs[thread_id].tick();
}

pub fn adjust_totals(part_id: usize, progress: u64) {
    let mut totals = TOTALS
        .lock()
        .expect("Failed to acquire TOTALS lock, lock poisoned!");

    totals[part_id] = progress;

    let total_progress = totals.iter().sum();
    PBRS.lock()
        .expect("Failed to acquire PBRS lock, lock poisoned!")[0]
        .set_position(total_progress);
}

pub fn update_bar(thread_id: usize, part_id: usize, progress: u64) {
    {
        PBRS.lock()
            .expect("Failed to acquire PBRS lock, lock poisoned!")[thread_id]
            .set_position(progress);
    }

    adjust_totals(part_id, progress);
}

pub fn success_bar(thread_id: usize) {
    finish_bar_with_message(thread_id, "Done");
}

pub fn fail_bar(thread_id: usize) {
    finish_bar_with_message(thread_id, "Download Failed!");
}

fn finish_bar_with_message(thread_id: usize, message: &str) {
    PBRS.lock()
        .expect("Failed to acquire PBRS lock, lock poisoned!")[thread_id]
        .finish_with_message(message);
}

fn build_bar(mb: &mut MultiProgress, style: ProgressStyle, size: u64) {
    let mut pbrs = PBRS
        .lock()
        .expect("Failed to acquire PBRS lock, lock poisoned!");
    let pb = mb.add(ProgressBar::new(size));
    pb.set_style(style);
    pb.tick();
    pbrs.push(pb);
}
