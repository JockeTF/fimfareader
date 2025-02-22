//! Query optimizer.

use chrono::prelude::*;
use dateparser::parse_with_timezone;
use regex::RegexBuilder;
use regex::escape;

use fimfareader::error::Error;
use fimfareader::error::Result;

use crate::parser::DateOpt;
use crate::parser::Field;
use crate::parser::Filter;
use crate::parser::Op;
use crate::parser::Source;

macro_rules! ok {
    ($func:expr) => {
        Ok(Box::new($func))
    };
}

pub fn optimize(src: Source, op: Op, value: &str) -> Result<Filter> {
    match src {
        Source::Str(f) => str(f, op, value),
        Source::Int(f) => int(f, op, value),
        Source::Dto(f) => dto(f, op, value),
    }
}

fn str(f: Field<Box<str>>, op: Op, value: &str) -> Result<Filter> {
    let exact: Box<str> = value.into();

    let result = RegexBuilder::new(&escape(value))
        .case_insensitive(true)
        .size_limit(1_048_576)
        .build();

    let Ok(regex) = result else {
        return Err(Error::query("Invalid value for fuzzy match"));
    };

    match op {
        Op::Exact => ok!(move |s| *f(s) == exact),
        Op::Fuzzy => ok!(move |s| regex.is_match(f(s))),
        _ => Err(Error::query("Invalid operation for text type")),
    }
}

fn int(f: Field<i32>, op: Op, value: &str) -> Result<Filter> {
    let Ok(value) = value.parse() else {
        return Err(Error::query("Invalid value for number type"));
    };

    match op {
        Op::Exact => ok!(move |s| *f(s) == value),
        Op::Fuzzy => ok!(move |s| *f(s) == value),
        Op::LessThan => ok!(move |s| *f(s) < value),
        Op::MoreThan => ok!(move |s| *f(s) > value),
    }
}

fn dto(f: Field<DateOpt>, op: Op, value: &str) -> Result<Filter> {
    let Ok(value) = parse_with_timezone(value, &Local) else {
        return Err(Error::query("Invalid value for date type"));
    };

    match op {
        Op::Exact => ok!(move |s| match f(s) {
            Some(dt) => *dt == value,
            None => false,
        }),
        Op::Fuzzy => ok!(move |s| match f(s) {
            Some(dt) => dt.date_naive() == value.date_naive(),
            None => false,
        }),
        Op::LessThan => ok!(move |s| match f(s) {
            Some(dt) => *dt < value,
            None => false,
        }),
        Op::MoreThan => ok!(move |s| match f(s) {
            Some(dt) => *dt > value,
            None => false,
        }),
    }
}
