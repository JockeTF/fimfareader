//! Main module.

pub mod archive;
pub mod error;

use std::env::args;
use std::time::Instant;

use crate::archive::Fetcher;
use crate::error::{Error, Result};

fn main() -> Result<()> {
    use Error::*;

    let argv = args().collect::<Vec<String>>();

    let path = match argv.len() {
        2 => Ok(argv.get(1).unwrap()),
        _ => Err(UserError("Usage: fimfareader <ARCHIVE>")),
    }?;

    println!("Hellopaca, World!");

    let start = Instant::now();
    let fetcher = Fetcher::from(path)?;
    let finish = Instant::now() - start;

    println!("Finished loading in {} milliseconds.", finish.as_millis());
    println!("The archive contains {} stories.", fetcher.iter().count());

    Ok(())
}
