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
        let start = Instant::now();

        let result = searcher
            .search(&line)
            .into_iter()
            .filter(|(_sid, score)| *score > 10f32)
            .filter_map(|(sid, score)| Some((fetcher.fetch(sid)?, score)))
            .collect::<Vec<_>>();

        let finish = (Instant::now() - start).as_millis();
        let count = result.len();

        println!("Found {} stories in {} milliseconds!", count, finish);

        for (story, score) in result {
            let key = &story.id;
            let title = &story.title;

            let tags = story
                .tags
                .iter()
                .map(|tag| String::from(&tag.name))
                .collect::<Vec<_>>()
                .join(", ");

            println!("{:02.02}% [{:>6}] {}", score, key, title);
            println!("{}", tags);
            println!("{}", story.short_description);
            println!("{}", story.url);
            println!();
        }
    }
}
