//! Story meta.

use chrono::prelude::*;
use lazy_static::lazy_static;
use serde::de::Error;
use serde::Deserialize;
use serde::Deserializer;
use serde_json::Value;

use super::interner::Interner;

lazy_static! {
    static ref TAGS: Interner<Tag> = Interner::new();
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Story {
    pub archive: Archive,
    pub author: Author,
    pub chapters: Vec<Chapter>,
    pub color: Option<Color>,
    pub completion_status: CompletionStatus,
    pub content_rating: ContentRating,
    pub cover_image: Option<CoverImage>,
    pub date_modified: Option<DateTime<Utc>>,
    pub date_published: Option<DateTime<Utc>>,
    pub date_updated: Option<DateTime<Utc>>,
    #[serde(deserialize_with = "null_to_html")]
    pub description_html: String,
    pub id: i64,
    pub num_chapters: i32,
    pub num_comments: i32,
    pub num_dislikes: i32,
    pub num_likes: i32,
    pub num_views: i32,
    pub num_words: i32,
    pub prequel: Option<i64>,
    pub published: bool,
    pub rating: i32,
    #[serde(deserialize_with = "null_to_text")]
    pub short_description: String,
    pub status: Status,
    pub submitted: bool,
    #[serde(deserialize_with = "tags_as_static")]
    pub tags: Vec<&'static Tag>,
    #[serde(deserialize_with = "null_to_text")]
    pub title: String,
    pub total_num_views: i32,
    pub url: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Archive {
    pub date_checked: Option<DateTime<Utc>>,
    pub date_created: Option<DateTime<Utc>>,
    pub date_fetched: Option<DateTime<Utc>>,
    pub date_updated: Option<DateTime<Utc>>,
    pub path: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Author {
    pub avatar: Option<Avatar>,
    pub bio_html: Option<String>,
    pub date_joined: Option<DateTime<Utc>>,
    #[serde(deserialize_with = "string_to_id")]
    pub id: i64,
    pub name: String,
    pub num_blog_posts: Option<i32>,
    pub num_followers: Option<i32>,
    pub num_stories: Option<i32>,
    pub url: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Avatar {
    #[serde(rename = "16")]
    pub x16: Option<String>,
    #[serde(rename = "32")]
    pub x32: Option<String>,
    #[serde(rename = "48")]
    pub x48: Option<String>,
    #[serde(rename = "64")]
    pub x64: Option<String>,
    #[serde(rename = "96")]
    pub x96: Option<String>,
    #[serde(rename = "128")]
    pub x128: Option<String>,
    #[serde(rename = "160")]
    pub x160: Option<String>,
    #[serde(rename = "192")]
    pub x192: Option<String>,
    #[serde(rename = "256")]
    pub x256: Option<String>,
    #[serde(rename = "320")]
    pub x320: Option<String>,
    #[serde(rename = "384")]
    pub x384: Option<String>,
    #[serde(rename = "512")]
    pub x512: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chapter {
    pub chapter_number: i32,
    pub date_modified: Option<DateTime<Utc>>,
    pub date_published: Option<DateTime<Utc>>,
    pub id: i64,
    pub num_views: i32,
    pub num_words: i32,
    pub published: bool,
    #[serde(deserialize_with = "null_to_text")]
    pub title: String,
    pub url: String,
}

#[derive(Clone, Debug)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionStatus {
    Cancelled,
    Complete,
    #[serde(alias = "on hiatus")]
    Hiatus,
    Incomplete,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentRating {
    Everyone,
    Mature,
    Teen,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverImage {
    pub full: String,
    pub large: String,
    pub medium: String,
    pub thumbnail: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    ApproveQueue,
    NotVisible,
    PostQueue,
    Visible,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub old_id: String,
    pub r#type: String,
    pub url: String,
}

fn null_to_html<'de, D>(d: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::deserialize(d)? {
        Some(text) => Ok(text),
        None => Ok(String::from("<p></p>")),
    }
}

fn null_to_text<'de, D>(d: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::deserialize(d)? {
        Some(text) => Ok(text),
        None => Ok(String::from("")),
    }
}

fn string_to_id<'de, D>(d: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    match Value::deserialize(d)? {
        Value::Number(value) => match value.as_i64() {
            Some(value) => Ok(value),
            _ => Err(Error::custom("Could not parse ID number")),
        },
        Value::String(value) => match value.parse::<i64>() {
            Ok(value) => Ok(value),
            _ => Err(Error::custom("Could not parse ID string")),
        },
        _ => Err(Error::custom("Invalid type for ID value")),
    }
}

fn tags_as_static<'de, D>(d: D) -> Result<Vec<&'static Tag>, D::Error>
where
    D: Deserializer<'de>,
{
    let tags: Vec<Tag> = Vec::deserialize(d)?;

    Ok(tags.into_iter().map(|tag| TAGS.intern(tag)).collect())
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(d: D) -> Result<Color, D::Error>
    where
        D: Deserializer<'de>,
    {
        let object = Value::deserialize(d)?;

        let text = object
            .get("hex")
            .and_then(|value| value.as_str())
            .ok_or_else(|| Error::custom("Color is missing hex value"))?;

        let Ok(array) = hex::decode(text) else {
            return Err(Error::custom("Color hex has invalid value"));
        };

        match array[..] {
            [red, green, blue] => Ok(Color { red, green, blue }),
            _ => Err(Error::custom("Color hex has invalid length")),
        }
    }
}
