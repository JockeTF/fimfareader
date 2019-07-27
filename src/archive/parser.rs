//! Index parser.

use std::io::BufRead;
use std::sync::mpsc::{channel, Receiver};
use std::thread::spawn;

use serde::de::Error;
use serde_json::error::Result;
use serde_json::from_str;

use super::story::Story;

const TRIM: &[char] = &['"', ',', ' ', '\t', '\n', '\r'];

pub fn parse(reader: impl BufRead) -> Result<Vec<Story>> {
    let (tx, rx) = channel();
    let rx = spawn_parser(rx);

    for line in reader.lines() {
        let line = line.map_err(|e| match e {
            _ => Error::custom("Could not read line"),
        })?;

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

    rx.recv().map_err(|e| match e {
        _ => Error::custom("Missing parser result"),
    })?
}

fn spawn_parser(stream: Receiver<String>) -> Receiver<Result<Vec<Story>>> {
    let (tx, rx) = channel();

    spawn(move || {
        let mut stories = Vec::with_capacity(250_000);
        let mut wrappers = String::with_capacity(2);

        while let Ok(line) = stream.recv() {
            if line.len() == 1 {
                wrappers.push_str(&line);
                continue;
            }

            match deserialize(line) {
                Ok(story) => stories.push(story),
                Err(e) => return tx.send(Err(e)),
            };
        }

        let count = stories.len();

        stories.sort_by_key(|story| story.id);
        stories.dedup_by_key(|story| story.id);
        stories.shrink_to_fit();

        if wrappers != "{}" {
            return tx.send(Err(Error::custom("Invalid file structure")));
        }

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
