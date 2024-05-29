//! Main module.

use std::env::args;
use std::error::Error;
use std::result::Result;
use std::time::Instant;

use fimfareader::archive::Fetcher;
use fimfareader_query::parse;
use rustyline::DefaultEditor;

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

    while let Ok(line) = editor.readline(">>> ") {
        editor.add_history_entry(&line)?;

        let filter = match parse(&line) {
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

    Ok(())
}
