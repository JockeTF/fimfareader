//! Index parser.

use std::io::BufRead;
use std::sync::mpsc::{channel, Receiver};
use std::thread::spawn;

use serde_json::from_str;

use crate::error::{Error, Result};
use crate::story::Story;

const TRIM: &[char] = &['"', ',', ' ', '\t', '\n', '\r'];

pub fn parse(reader: impl BufRead) -> Result<Vec<Story>> {
    use Error::*;

    let (tx, rx) = channel();
    let rx = spawn_parser(rx);

    for line in reader.lines() {
        let line = line.map_err(|e| match e {
            _ => SourceError("Could not read index line."),
        })?;

        if tx.send(line).is_ok() {
            continue;
        }

        return Err(match rx.recv() {
            Err(_) => SourceError("Parser disappeared unexpectedly."),
            Ok(Ok(_)) => SourceError("Parser returned unexpectedly."),
            Ok(Err(error)) => error,
        });
    }

    drop(tx);

    rx.recv().map_err(|e| match e {
        _ => SourceError("Missing parser result."),
    })?
}

fn spawn_parser(stream: Receiver<String>) -> Receiver<Result<Vec<Story>>> {
    use Error::*;

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
            return tx.send(Err(SourceError("Invalid index structure.")));
        }

        if count != stories.len() {
            return tx.send(Err(SourceError("Index contains duplicates.")));
        }

        tx.send(Ok(stories))
    });

    rx
}

fn deserialize(line: String) -> Result<Story> {
    use Error::*;

    let split = line
        .splitn(2, ':')
        .map(|value| value.trim_matches(TRIM))
        .collect::<Vec<&str>>();

    let (skey, json) = match split[..] {
        [skey, json] => Ok((skey, json)),
        _ => Err(SourceError("Invalid line format.")),
    }?;

    let key: i64 = skey.parse().map_err(|e| match e {
        _ => SourceError("Invalid meta key."),
    })?;

    let story: Story = from_str(json).map_err(|e| match e {
        _ => SourceError("Invalid meta value."),
    })?;

    if key != story.id {
        return Err(SourceError("Meta key mismatch."));
    }

    Ok(story)
}
