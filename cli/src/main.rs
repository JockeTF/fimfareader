//! Main module.

use std::env::args;
use std::fs::File;
use std::io::BufReader;
use std::time::Instant;

use rustyline::Editor;

use zip::ZipArchive;

use fimfareader::prelude::*;
use fimfareader_search::Searcher;

fn exit(error: Error) -> ! {
    eprintln!("{}", error);

    std::process::exit(1)
}

fn main() {
    let argv = args().collect::<Vec<String>>();
    let mut editor = Editor::<()>::new();

    if argv.len() != 2 {
        eprintln!("Usage: fimfareader <ARCHIVE>");
        std::process::exit(1);
    }

    println!("Hellopaca, World!");

    let start = Instant::now();
    let result = Fetcher::new(&argv[1]);
    let fetcher = result.map_err(exit).unwrap();
    let finish = (Instant::now() - start).as_millis();
    let count = fetcher.iter().count();

    println!("Finished loading in {} milliseconds.", finish);
    println!("The archive contains {} stories.", count);

    let opener = || {
        let file = File::open(&argv[1]).unwrap();
        let reader = BufReader::new(file);

        ZipArchive::new(reader).unwrap()
    };

    let searcher = Searcher::new(&fetcher, &opener);

    while let Ok(line) = editor.readline(">>> ") {
        editor.add_history_entry(&line);

        let filter = searcher.parse(&line);

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
