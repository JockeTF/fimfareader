use std::collections::HashMap;
use std::env::args;
use std::io::Cursor;
use std::io::Read;

use chrono::prelude::*;
use fimfareader::prelude::*;
use rayon::prelude::*;

use indicatif::ParallelProgressIterator;

use regex::Regex;
use regex::RegexBuilder;

use zip::ZipArchive;

// TODO: More varied and less literal matches.
const PATTERN: &str = "[^a-z]hug(s|ged|ging)?[^a-z]";

struct Stat {
    date: DateTime<Utc>,
    count: u64,
    words: u64,
}

fn count(regex: &Regex, story: &Story, data: Vec<u8>) -> Vec<Stat> {
    let mut archive = ZipArchive::new(Cursor::new(data)).unwrap();

    // TODO: Statistics per chapter.
    let date = match story.date_published {
        Some(published) => published,
        None => return Vec::new(),
    };

    let mut matches = 0;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let mut data = String::with_capacity(file.size() as usize);

        file.read_to_string(&mut data).unwrap();
        matches += regex.find_iter(&data).count();
    }

    let count = matches as u64;
    let words = story.num_words as u64;

    vec![Stat { date, count, words }]
}

fn main() {
    let argv = args().collect::<Vec<String>>();

    if argv.len() != 2 {
        eprintln!("Usage: {} <ARCHIVE>", argv[0]);
        std::process::exit(1);
    }

    let fetcher = Fetcher::new(&argv[1]).unwrap();

    let pattern = RegexBuilder::new(PATTERN)
        .case_insensitive(true)
        .build()
        .unwrap();

    let stats = fetcher
        .index()
        .par_iter()
        .progress_count(fetcher.index().len() as u64)
        .map(|story| (story, fetcher.read(story).unwrap()))
        .flat_map_iter(|(story, data)| count(&pattern, story, data))
        .collect::<Vec<Stat>>();

    // TODO: Finer granularity for better graphing.
    let mut yearly = HashMap::<i32, (u64, u64)>::new();

    for stat in stats {
        let year = stat.date.year();
        let value = yearly.remove(&year).unwrap_or_else(|| (0, 0));

        yearly.insert(year, (value.0 + stat.count, value.1 + stat.words));
    }

    let mut yearly = yearly.into_iter().collect::<Vec<_>>();

    yearly.sort();

    for (year, (count, words)) in yearly.into_iter() {
        let modifier = 1_000_000f64 / words as f64;
        let hpm = modifier * count as f64;

        println!("{year}: {hpm:.04}");
    }
}
