use std::collections::vec_deque::VecDeque;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

pub fn read_file(file_name: &str, queue: &mut VecDeque<String>) {
    // where
    //     T: FromIterator<String>,
    let f = File::open(file_name).expect("Failed to open File {file_name}");
    let reader: BufReader<File> = BufReader::new(f);

    for line in reader.lines() {
        let line = line.expect("Failed to read line");
        queue.push_back(line);
    }
}
