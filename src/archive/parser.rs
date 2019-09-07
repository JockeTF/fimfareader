//! Index parser.

use std::io::BufRead;
use std::thread::spawn;

use serde::de::Error;
use serde_json::error::Result;
use serde_json::from_str;

use crossbeam_channel::{bounded, Receiver};

use super::story::Story;

const TRIM: &[char] = &['"', ',', ' ', '\t', '\n', '\r'];

pub fn parse(reader: impl BufRead) -> Result<Vec<Story>> {
    let mut wrappers = String::with_capacity(2);

    let (tx, rx) = bounded(4096);
    let rx = spawn_parser(rx);
    let rx = spawn_reducer(rx);

    for line in reader.lines() {
        let line = line.map_err(|e| match e {
            _ => Error::custom("Could not read line"),
        })?;

        if line.len() == 1 {
            wrappers.push_str(&line);
            continue;
        }

        if tx.send(line).is_ok() {
            continue;
        }

        return Err(match rx.recv() {
            Err(_) => Error::custom("Parser disappeared unexpectedly"),
            Ok(Ok(_)) => Error::custom("Parser returned unexpectedly"),
            Ok(Err(error)) => error,
        });
    }

    drop(tx);

    if wrappers != "{}" {
        return Err(Error::custom("Invalid file structure"));
    }

    rx.recv().map_err(|e| match e {
        _ => Error::custom("Missing parser result"),
    })?
}

fn spawn_parser(ix: Receiver<String>) -> Receiver<Result<Story>> {
    let (tx, rx) = bounded(4096);

    for _ in 0..2 {
        let stream = ix.clone();
        let result = tx.clone();

        spawn(move || {
            while let Ok(line) = stream.recv() {
                match result.send(deserialize(line)) {
                    Err(e) => return Err(e),
                    Ok(()) => continue,
                }
            }

            Ok(())
        });
    }

    rx
}

fn spawn_reducer(ix: Receiver<Result<Story>>) -> Receiver<Result<Vec<Story>>> {
    let (tx, rx) = bounded(1024);

    spawn(move || {
        let mut stories = Vec::with_capacity(250_000);

        while let Ok(story) = ix.recv() {
            match story {
                Ok(story) => stories.push(story),
                Err(e) => return tx.send(Err(e)),
            }
        }

        let count = stories.len();

        stories.sort_by_key(|story| story.id);
        stories.dedup_by_key(|story| story.id);
        stories.shrink_to_fit();

        if count != stories.len() {
            return tx.send(Err(Error::custom("Found duplicate story")));
        }

        tx.send(Ok(stories))
    });

    rx
}

fn deserialize(line: String) -> Result<Story> {
    let split = line
        .splitn(2, ':')
        .map(|value| value.trim_matches(TRIM))
        .collect::<Vec<&str>>();

    let (skey, json) = match split[..] {
        [skey, json] => Ok((skey, json)),
        _ => Err(Error::custom("Invalid line format")),
    }?;

    let story: Story = from_str(json)?;

    let key: i64 = skey.parse().map_err(|e| match e {
        _ => Error::custom("Invalid line key"),
    })?;

    if key != story.id {
        return Err(Error::custom("Line key mismatch"));
    }

    Ok(story)
}
