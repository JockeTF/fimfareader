//! Index parser.

use std::collections::HashSet;
use std::io::BufRead;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::thread::spawn;

use rayon::prelude::*;
use serde::de::Error;
use serde_json::error::Result;
use serde_json::from_str;

use super::story::{Author, Story, Tag};

const TRIM: &[char] = &['"', ',', ' ', '\t', '\n', '\r'];

pub fn parse(reader: impl BufRead) -> Result<Vec<Story>> {
    let mut wrappers = String::with_capacity(2);

    let (tx, rx) = channel();
    let rx = spawn_parser(rx);

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

    let result = rx.recv().map_err(|e| match e {
        _ => Error::custom("Missing parser result"),
    })?;

    Ok(dedup(result?))
}

fn dedup(mut stories: Vec<Story>) -> Vec<Story> {
    let mut authors: HashSet<Arc<Author>> = HashSet::new();
    let mut tags: HashSet<Arc<Tag>> = HashSet::new();

    for story in stories.iter_mut() {
        if let Some(author) = authors.get(&story.author) {
            story.author = author.clone();
        } else {
            authors.insert(story.author.clone());
        }
    }

    for story in stories.iter_mut() {
        let unseen = story
            .tags
            .iter()
            .filter(|tag| !tags.contains(*tag))
            .map(|tag| tag.clone())
            .collect::<Vec<_>>();

        tags.extend(unseen);

        story.tags = story
            .tags
            .iter()
            .filter_map(|tag| tags.get(tag))
            .map(|tag| tag.clone())
            .collect();
    }

    for tag in tags.iter() {
        println!("{}: {}", tag.name, Arc::strong_count(tag));
    }

    stories
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

    let key: i64 = skey.parse().map_err(|e| match e {
        _ => Error::custom("Invalid line key"),
    })?;

    if key != story.id {
        return Err(Error::custom("Line key mismatch"));
    }

    Ok(story)
}
