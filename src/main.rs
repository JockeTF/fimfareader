//! Main module.

pub mod archive;
pub mod error;
pub mod query;

use std::env::args;
use std::io::stdin;
use std::io::stdout;
use std::io::Write;
use std::time::Instant;

use crate::archive::Fetcher;
use crate::error::Error;
use crate::query::parse;

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

fn main() {
    let argv = args().collect::<Vec<String>>();

    if argv.len() != 2 {
        eprintln!("Usage: fimfareader <ARCHIVE>");
        std::process::exit(1);
    }

    println!("Hellopaca, World!");

    let start = Instant::now();
    let result = Fetcher::from(&argv[1]);
    let fetcher = result.map_err(exit).unwrap();
    let finish = (Instant::now() - start).as_millis();
    let count = fetcher.iter().count();

    println!("Finished loading in {} milliseconds.", finish);
    println!("The archive contains {} stories.", count);

    loop {
        let filter = match parse(&input()) {
            Ok(filter) => filter,
            Err(error) => {
                println!("{}", error);
                continue;
            }
        };

        let start = Instant::now();
        let stories = fetcher.filter(&filter);
        let finish = (Instant::now() - start).as_millis();
        let count = stories.len();

        println!("Found {} stories in {} milliseconds!", count, finish);

        if count > 32 {
            continue;
        }

        for story in stories.iter() {
            let key = &story.id;
            let title = &story.title;

            println!("[{}] {}", key, title);
        }
    }
}
