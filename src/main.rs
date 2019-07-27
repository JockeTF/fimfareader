//! Main module.

pub mod archive;
pub mod error;

use std::env::args;
use std::time::Instant;

use crate::archive::Fetcher;
use crate::error::Error;

fn exit(error: Error) -> ! {
    eprintln!("{}", error);

    std::process::exit(1)
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
    let finish = Instant::now() - start;

    let fetcher = result.map_err(exit).unwrap();

    println!("Finished loading in {} milliseconds.", finish.as_millis());
    println!("The archive contains {} stories.", fetcher.iter().count());
}
