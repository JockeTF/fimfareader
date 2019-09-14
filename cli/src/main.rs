//! Main module.

use std::env::args;
use std::io::prelude::*;
use std::io::stdin;
use std::io::stdout;
use std::io::Write;
use std::time::Instant;

use indicatif::ProgressIterator;

use fimfareader::prelude::*;

fn exit(error: Error) -> ! {
    eprintln!("{}", error);

    std::process::exit(1)
}

fn input() -> String {
    let mut buffer = String::new();

    print!(">>> ");
    stdout().flush().unwrap();
    stdin().read_line(&mut buffer).unwrap();

    buffer
}

fn extract(data: Vec<u8>) -> u64 {
    use std::io::Cursor;
    use zip::read::ZipArchive;

    let curs = Cursor::new(data);
    let mut zobj = ZipArchive::new(curs).unwrap();
    let mut result = 0;

    for i in 0..zobj.len() {
        let mut entry = zobj.by_index(i).unwrap();
        let count = entry.size() as usize;
        let mut data: Vec<u8> = Vec::with_capacity(count);
        let read = entry.read_to_end(&mut data).unwrap();

        if count != read {
            panic!(format!("Expected {}, got {}.", count, read));
        }

        result += read as u64;
    }

    result
}

fn main() {
    let argv = args().collect::<Vec<String>>();

    if argv.len() != 2 {
        eprintln!("Usage: fimfareader <ARCHIVE>");
        std::process::exit(1);
    }

    println!("Hellopaca, World!");

    let result = Fetcher::from(&argv[1]);
    let fetcher = result.map_err(exit).unwrap();
    let start = Instant::now();

    let bytes: u64 = fetcher
        .iter()
        .progress()
        .map(|story| fetcher.read(story))
        .map(|result| result.unwrap())
        .map(|vector| extract(vector))
        .sum();

    let finish = (Instant::now() - start).as_secs();

    println!("Extracted {} in {} seconds!", bytes, finish);
}
