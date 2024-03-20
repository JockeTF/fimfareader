//! Query parser.

use chrono::DateTime;
use chrono::Utc;

use nom::branch::alt;
use nom::bytes::complete::escaped;
use nom::bytes::complete::tag;
use nom::character::complete::char;
use nom::character::complete::none_of;
use nom::character::complete::one_of;
use nom::character::complete::space0;
use nom::combinator::eof;
use nom::combinator::map;
use nom::combinator::value;
use nom::error::Error as NomError;
use nom::error::ErrorKind as NomErrorKind;
use nom::multi::separated_list1;
use nom::sequence::delimited;
use nom::sequence::preceded;
use nom::sequence::terminated;
use nom::sequence::tuple;
use nom::Err as NomErr;
use nom::Finish;
use nom::IResult;

use fimfareader::archive::Story;
use fimfareader::error::*;

use super::optimizer::optimize;

type Filter = Box<dyn Fn(&Story) -> bool + Sync>;

pub enum Source {
    IntFn(Box<dyn Fn(&Story) -> i64 + Sync>),
    StrFn(Box<dyn Fn(&Story) -> &str + Sync>),
    DtuFn(Box<dyn Fn(&Story) -> &Option<DateTime<Utc>> + Sync>),
}

#[derive(Clone)]
pub enum Operator {
    Exact,
    Fuzzy,
    LessThan,
    MoreThan,
}

macro_rules! sfn {
    ($tag:expr => $func:expr) => {
        preceded(tag($tag), |input| {
            Ok((input, Source::StrFn(Box::new($func))))
        })
    };
}

macro_rules! ifn {
    ($tag:expr => $func:expr) => {{
        preceded(tag($tag), |input| {
            Ok((input, Source::IntFn(Box::new($func))))
        })
    }};
}

macro_rules! dfn {
    ($tag:expr => $func:expr) => {
        preceded(tag($tag), |input| {
            Ok((input, Source::DtuFn(Box::new($func))))
        })
    };
}

fn source(input: &str) -> IResult<&str, Source> {
    let story = alt((
        ifn!("id" => |s| s.id),
        sfn!("story" => |s| &s.title),
        sfn!("title" => |s| &s.title),
        sfn!("description" => |s| &s.description_html),
        sfn!("short description" => |s| &s.short_description),
        sfn!("url" => |s| &s.url),
        dfn!("modified" => |s| &s.date_modified),
        dfn!("published" => |s| &s.date_published),
        dfn!("updated" => |s| &s.date_updated),
        ifn!("chapters" => |s| i64::from(s.num_chapters)),
        ifn!("comments" => |s| i64::from(s.num_comments)),
        ifn!("dislikes" => |s| i64::from(s.num_dislikes)),
        ifn!("likes" => |s| i64::from(s.num_likes)),
        ifn!("total views" => |s| i64::from(s.total_num_views)),
        ifn!("views" => |s| i64::from(s.num_views)),
        ifn!("words" => |s| i64::from(s.num_words)),
    ));

    let author = alt((
        sfn!("author" => |s| &s.author.name),
        sfn!("author name" => |s| &s.author.name),
        ifn!("author id" => |s| s.author.id),
        dfn!("author joined" => |s| &s.author.date_joined),
    ));

    let archive = alt((
        sfn!("path" => |s| &s.archive.path),
        sfn!("archive" => |s| &s.archive.path),
        sfn!("archive path" => |s| &s.archive.path),
        dfn!("entry checked" => |s| &s.archive.date_checked),
        dfn!("entry created" => |s| &s.archive.date_created),
        dfn!("entry fetched" => |s| &s.archive.date_fetched),
        dfn!("entry updated" => |s| &s.archive.date_updated),
    ));

    preceded(space0, alt((story, author, archive)))(input)
}

fn operator(input: &str) -> IResult<&str, Operator> {
    let operator = alt((
        value(Operator::Exact, char('=')),
        value(Operator::Fuzzy, char(':')),
        value(Operator::LessThan, char('<')),
        value(Operator::MoreThan, char('>')),
    ));

    preceded(space0, operator)(input)
}

fn unescape(input: &str) -> String {
    input
        .replace("\\)", ")")
        .replace("\\,", ",")
        .replace("\\|", "|")
        .replace("\\\\", "\\")
}

fn evalue(input: &str) -> IResult<&str, &str> {
    escaped(none_of("),|\\"), '\\', one_of("),|\\"))(input)
}

fn target(input: &str) -> IResult<&str, String> {
    preceded(space0, map(evalue, |value| unescape(value.trim())))(input)
}

fn item(input: &str) -> IResult<&str, Filter> {
    let result = tuple((source, operator, target))(input)?;
    let (left, (src, op, value)) = result;

    let Ok(filter) = optimize(src, op, &value) else {
        let error = NomError::new(input, NomErrorKind::Permutation);
        return Err(NomErr::Failure(error));
    };

    Ok((left, filter))
}

fn parens(input: &str) -> IResult<&str, Filter> {
    let group = delimited(
        preceded(space0, char('(')),
        preceded(space0, ofunc),
        preceded(space0, char(')')),
    );

    alt((group, item))(input)
}

fn negate(input: &str) -> IResult<&str, Filter> {
    let (input, filter) = parens(input)?;
    Ok((input, Box::new(move |s| !filter(s))))
}

fn nlist(input: &str) -> IResult<&str, Filter> {
    let negated = preceded(char('!'), negate);
    preceded(space0, alt((negated, parens)))(input)
}

fn alist(input: &str) -> IResult<&str, Vec<Filter>> {
    let sep = preceded(space0, char(','));
    separated_list1(sep, nlist)(input)
}

fn afunc(input: &str) -> IResult<&str, Filter> {
    let (left, mut filters) = alist(input)?;

    if filters.len() == 1 {
        return Ok((left, filters.remove(0)));
    }

    let filter: Filter = Box::new(move |story| {
        for filter in filters.iter() {
            if !filter(story) {
                return false;
            }
        }

        true
    });

    Ok((left, filter))
}

fn olist(input: &str) -> IResult<&str, Vec<Filter>> {
    let sep = preceded(space0, char('|'));
    separated_list1(sep, afunc)(input)
}

fn ofunc(input: &str) -> IResult<&str, Filter> {
    let (left, mut filters) = olist(input)?;

    if filters.len() == 1 {
        return Ok((left, filters.remove(0)));
    }

    let filter: Filter = Box::new(move |story| {
        for filter in filters.iter() {
            if filter(story) {
                return true;
            }
        }

        false
    });

    Ok((left, filter))
}

fn complete(input: &str) -> IResult<&str, Filter> {
    terminated(ofunc, eof)(input.trim())
}

pub fn parse(query: &str) -> Result<Filter> {
    match complete(query).finish() {
        Ok((_, filter)) => Ok(filter),
        Err(e) => Err(Error::query(e)),
    }
}
