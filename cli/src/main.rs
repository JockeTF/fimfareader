//! Main module.

use std::env::args;
use std::error::Error;
use std::result::Result;
use std::time::Instant;

use rustyline::DefaultEditor;

use fimfareader::archive::Fetcher;
use fimfareader_search::Searcher;

fn main() -> Result<(), Box<dyn Error>> {
    let argv = args().collect::<Vec<String>>();
    let mut editor = DefaultEditor::new()?;

    if argv.len() != 2 {
        eprintln!("Usage: fimfareader <ARCHIVE>");
        std::process::exit(1);
    }

    println!("Hellopaca, World!");

    let start = Instant::now();
    let fetcher = Fetcher::new(&argv[1])?;
    let finish = Instant::now() - start;
    let count = fetcher.iter().count();

    println!("Finished loading in {finish:?}.");
    println!("The archive contains {count} stories.");

    let searcher = Searcher::new(&fetcher);

    while let Ok(line) = editor.readline(">>> ") {
        editor.add_history_entry(&line)?;
        let start = Instant::now();

        let result = searcher
            .search(&line)
            .into_iter()
            .filter(|(_sid, score)| *score > 10f32)
            .map(|(sid, score)| (i32::try_from(sid).unwrap(), score))
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
                .map(|tag| tag.name.to_string())
                .collect::<Vec<_>>()
                .join(", ");

            println!("{:02.02}% [{:>6}] {}", score, key, title);
            println!("{}", tags);
            println!("{}", story.short_description);
            println!("{}", story.url);
            println!();
        }
    }

    Ok(())
}
