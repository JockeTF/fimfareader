//! Query parser.

use chrono::prelude::*;

use nom::character::complete::*;
use nom::error::ErrorKind as NomErrorKind;
use nom::sequence::*;
use nom::*;

use fimfareader::archive::Story;
use fimfareader::error::*;

use super::optimizer::optimize;

type Filter = Box<dyn Fn(&Story) -> bool + Sync>;

pub enum Source {
    IntFn(Box<dyn Fn(&Story) -> i64 + Sync>),
    StrFn(Box<dyn Fn(&Story) -> &str + Sync>),
    DtuFn(Box<dyn Fn(&Story) -> &Option<DateTime<Utc>> + Sync>),
}

pub enum Operator {
    Exact,
    Fuzzy,
    LessThan,
    MoreThan,
}

macro_rules! sfn {
    ($func:expr) => {
        |_| Source::StrFn(Box::new($func))
    };
}

macro_rules! ifn {
    ($func:expr) => {
        |_| Source::IntFn(Box::new($func))
    };
}

macro_rules! dfn {
    ($func:expr) => {
        |_| Source::DtuFn(Box::new($func))
    };
}

named!(source<&str, Source>, preceded!(space0, alt!(
    tag!("id") => { ifn!(|s| s.id) } |

    tag!("story") => { sfn!(|s| &s.title) } |
    tag!("title") => { sfn!(|s| &s.title) } |

    tag!("description") => { sfn!(|s| &s.description_html) } |
    tag!("short description") => { sfn!(|s| &s.short_description) } |
    tag!("url") => { sfn!(|s| &s.url) } |

    tag!("modified") => { dfn!(|s| &s.date_modified) } |
    tag!("published") => { dfn!(|s| &s.date_published) } |
    tag!("updated") => { dfn!(|s| &s.date_updated) } |

    tag!("chapters") => { ifn!(|s| i64::from(s.num_chapters)) } |
    tag!("comments") => { ifn!(|s| i64::from(s.num_comments)) } |
    tag!("dislikes") => { ifn!(|s| i64::from(s.num_dislikes)) } |
    tag!("likes") => { ifn!(|s| i64::from(s.num_likes)) } |
    tag!("total views") => { ifn!(|s| i64::from(s.total_num_views)) } |
    tag!("views") => { ifn!(|s| i64::from(s.num_views)) } |
    tag!("words") => { ifn!(|s| i64::from(s.num_words)) } |

    tag!("author") => { sfn!(|s| &s.author.name) } |
    tag!("author name") => { sfn!(|s| &s.author.name) } |

    tag!("author id") => { ifn!(|s| s.author.id) } |
    tag!("author joined") => { dfn!(|s| &s.author.date_joined) } |

    tag!("path") => { sfn!(|s| &s.archive.path) } |
    tag!("archive") => { sfn!(|s| &s.archive.path) } |
    tag!("archive path") => { sfn!(|s| &s.archive.path) } |

    tag!("entry checked") => { dfn!(|s| &s.archive.date_checked) } |
    tag!("entry created") => { dfn!(|s| &s.archive.date_created) } |
    tag!("entry fetched") => { dfn!(|s| &s.archive.date_fetched) } |
    tag!("entry updated") => { dfn!(|s| &s.archive.date_updated) }
)));

named!(operator<&str, Operator>, preceded!(space0, alt!(
    tag!("=") => { |_| Operator::Exact } |
    tag!(":") => { |_| Operator::Fuzzy } |
    tag!("<") => { |_| Operator::LessThan } |
    tag!(">") => { |_| Operator::MoreThan }
)));

fn unescape(input: &str) -> String {
    input
        .replace("\\)", ")")
        .replace("\\,", ",")
        .replace("\\|", "|")
        .replace("\\\\", "\\")
}

named!(value<&str, &str>,
    escaped!(none_of!("),|\\"), '\\', one_of!("),|\\"))
);

named!(target<&str, String>, preceded!(space0,
    map!(value, |value| unescape(value.trim()))
));

fn item(input: &str) -> IResult<&str, Filter> {
    let result = tuple((source, operator, target))(input)?;
    let (left, (src, op, value)) = result;

    let Ok(filter) = optimize(src, op, &value) else {
        return Err(Err::Failure((input, NomErrorKind::Permutation)));
    };

    Ok((left, filter))
}

named!(parens<&str, Filter>, alt!(
    delimited!(
        preceded!(space0, tag!("(")),
        preceded!(space0, call!(ofunc)),
        preceded!(space0, tag!(")"))
    ) |
    call!(item)
));

fn negate(input: &str) -> IResult<&str, Filter> {
    let (left, filter) = parens(input)?;

    Ok((left, Box::new(move |s| !filter(s))))
}

named!(nlist<&str, Filter>, preceded!(space0, alt!(
    preceded!(char('!'), call!(negate)) | call!(parens)
)));

named!(alist<&str, Vec<Filter>>, separated_nonempty_list!(
    preceded!(space0, char(',')), call!(nlist)
));

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

named!(olist<&str, Vec<Filter>>, separated_nonempty_list!(
    preceded!(space0, char('|')), call!(afunc)
));

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

fn format_error(query: &str, input: &str, error: NomErrorKind) -> String {
    let description = error.description().to_lowercase();
    let position = query.len() - input.len();

    format!("Invalid {} at {}", description, position)
}

fn translate_error(query: &str, error: Err<(&str, NomErrorKind)>) -> Error {
    let message = match error {
        Err::Error((i, e)) => format_error(query, i, e),
        Err::Failure((i, e)) => format_error(query, i, e),
        Err::Incomplete(_) => String::from("Incomplete input"),
    };

    Error::query(message)
}

fn translate_incomplete(query: &str, input: &str) -> Error {
    let position = query.len() - input.len();

    Error::query(format!("Incomplete input at {}", position))
}

pub fn parse(query: &str) -> Result<Filter> {
    match ofunc(query.trim()) {
        Ok(("", filter)) => Ok(filter),
        Ok((i, _)) => Err(translate_incomplete(query, i)),
        Err(e) => Err(translate_error(query, e)),
    }
}
