//! Query parser.

use chrono::DateTime;
use chrono::Utc;
use derive_more::From;

use nom::Err as NomErr;
use nom::Finish;
use nom::IResult;
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

use fimfareader::archive::Story;
use fimfareader::error::*;

use crate::optimizer::optimize;

pub(crate) type DateOpt = Option<DateTime<Utc>>;
pub(crate) type Field<T> = &'static (dyn Fn(&Story) -> &T + Sync);
pub(crate) type Filter = Box<dyn Fn(&Story) -> bool + Sync>;

#[derive(From)]
pub(crate) enum Source {
    Int(Field<i32>),
    Str(Field<Box<str>>),
    Dto(Field<DateOpt>),
}

#[derive(Clone)]
pub(crate) enum Op {
    Exact,
    Fuzzy,
    LessThan,
    MoreThan,
}

macro_rules! ext {
    ($($tag:literal => $($path:ident).+),+,) => {
        alt(($(preceded(tag($tag), |input| {
            let field: Field<_> = &|story| &story.$($path).+;
            Ok((input, Source::from(field)))
        })),+))
    };
}

fn source(input: &str) -> IResult<&str, Source> {
    let story = ext! {
        "id" => id,
        "url" => url,
        "story" => title,
        "title" => title,
        "description" => description_html,
        "short description" => short_description,
        "modified" => date_modified,
        "published" => date_published,
        "updated" => date_updated,
        "chapters" => num_chapters,
        "comments" => num_comments,
        "dislikes" => num_dislikes,
        "likes" => num_likes,
        "total views" => total_num_views,
        "views" => num_views,
        "words" => num_words,
    };

    let author = ext! {
        "author" => author.name,
        "author name" => author.name,
        "author id" => author.id,
        "author joined" => author.date_joined,
    };

    let archive = ext! {
        "path" => archive.path,
        "archive" => archive.path,
        "archive path" => archive.path,
        "entry checked" => archive.date_checked,
        "entry created" => archive.date_created,
        "entry fetched" => archive.date_fetched,
        "entry updated" => archive.date_updated,
    };

    preceded(space0, alt((story, author, archive)))(input)
}

fn operator(input: &str) -> IResult<&str, Op> {
    let operator = alt((
        value(Op::Exact, char('=')),
        value(Op::Fuzzy, char(':')),
        value(Op::LessThan, char('<')),
        value(Op::MoreThan, char('>')),
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
