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

        if 3 < line.len() {
            tx.send(line).unwrap();
        }
    }

    drop(tx);

    rx.recv().unwrap()
}

fn spawn_parser(stream: Receiver<String>) -> Receiver<Result<Vec<Story>>> {
    let (tx, rx) = channel();

    spawn(move || {
        let mut stories = Vec::with_capacity(250_000);

        while let Ok(line) = stream.recv() {
            match deserialize(line) {
                Ok(story) => stories.push(story),
                Err(e) => return tx.send(Err(e)).unwrap(),
            };
        }

        stories.shrink_to_fit();
        stories.sort_by_key(|story| story.id);

        tx.send(Ok(stories)).unwrap();
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
