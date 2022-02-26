use std::env::args;
use std::io::Cursor;
use std::io::Read;

use fimfareader::prelude::*;
use rayon::prelude::*;

use zip::ZipArchive;

#[allow(unused)]
#[derive(Debug)]
struct Stat {
    story: i64,
    chars: i64,
    count: i64,
}

fn count(story: &Story, data: Vec<u8>) -> Stat {
    let mut archive = ZipArchive::new(Cursor::new(data)).unwrap();

    let id = story.id;
    let mut count = 0;
    let mut chars = 0;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let mut data = String::with_capacity(file.size() as usize);

        let bytes = match file.read_to_string(&mut data) {
            Ok(bytes) => bytes,
            Err(_) => {
                return Stat {
                    story: id,
                    count: -1,
                    chars: -1,
                }
            }
        };

        let matches = data
            .chars()
            .enumerate()
            .filter(|(_, chr)| *chr == '\u{9d}')
            .map(|(index, _)| index)
            .collect::<Vec<usize>>();

        count += matches.len() as i64;
        chars += bytes as i64;

        for pos in matches {
            let min = pos.saturating_sub(32);

            let snip = data
                .chars()
                .skip(min)
                .take(64)
                .filter(|c| !c.is_whitespace() || *c == ' ')
                .collect::<String>();

            println!("[{id:>6}] {snip}");
        }
    }

    Stat {
        story: id,
        count,
        chars,
    }
}

fn main() {
    let argv = args().collect::<Vec<String>>();

    if argv.len() != 2 {
        eprintln!("Usage: {} <ARCHIVE>", argv[0]);
        std::process::exit(1);
    }

    let fetcher = Fetcher::new(&argv[1]).unwrap();

    let stats = fetcher
        .index()
        .par_iter()
        .map(|story| (story, fetcher.read(story).unwrap()))
        .map(|(story, data)| count(story, data))
        .filter(|stat| stat.count != 0)
        .collect::<Vec<_>>();

    for stat in stats {
        println!("{stat:?}");
    }
}
