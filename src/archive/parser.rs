//! Index parser.

use std::io::BufRead;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::thread::spawn;

use rayon::prelude::*;
use serde::de::Error;
use serde_json::error::Result;
use serde_json::from_str;

use super::story::Story;

const TRIM: &[char] = &['"', ',', ' ', '\t', '\n', '\r'];

pub fn parse(reader: impl BufRead) -> Result<Vec<Story>> {
    let mut wrappers = String::with_capacity(2);

    let (tx, rx) = channel();
    let rx = spawn_parser(rx);

    for line in reader.lines() {
        let Ok(line) = line else {
            return Err(Error::custom("Could not read line"));
        };

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

    let Ok(result) = rx.recv() else {
        return Err(Error::custom("Missing parser result"));
    };

    result
}

fn spawn_parser(stream: Receiver<String>) -> Receiver<Result<Vec<Story>>> {
    let (tx, rx) = channel();

    spawn(move || {
        let bridge = stream.into_iter().par_bridge();
        let result = bridge.map(deserialize).collect();

        let mut stories: Vec<Story> = match result {
            Err(e) => return tx.send(Err(e)),
            Ok(stories) => stories,
        };

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

    let Ok(key) = skey.parse::<i32>() else {
        return Err(Error::custom("Invalid line key"));
    };

    if key != story.id {
        return Err(Error::custom("Line key mismatch"));
    }

    Ok(story)
}
