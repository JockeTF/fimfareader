//! Query optimizer.

use chrono::prelude::*;

use chrono_english::{parse_date_string, Dialect};

use regex::RegexBuilder;
use regex::escape;

use fimfareader::archive::Story;
use fimfareader::error::{Error, Result};

use super::parser::{Operator, Source};

use Operator::*;
use Source::*;

type Filter = Box<dyn Fn(&Story) -> bool + Sync>;

type IntFn = Box<dyn Fn(&Story) -> i64 + Sync>;
type StrFn = Box<dyn Fn(&Story) -> &str + Sync>;
type DtuFn = Box<dyn Fn(&Story) -> &Option<DateTime<Utc>> + Sync>;

macro_rules! ok {
    ($func:expr) => {
        Ok(Box::new($func))
    };
}

pub fn optimize(src: Source, op: Operator, value: &str) -> Result<Filter> {
    match src {
        StrFn(f) => strfn(f, op, value),
        IntFn(f) => intfn(f, op, value),
        DtuFn(f) => dtufn(f, op, value),
    }
}

fn strfn(f: StrFn, op: Operator, value: &str) -> Result<Filter> {
    let exact: String = value.into();

    let result = RegexBuilder::new(&escape(value))
        .case_insensitive(true)
        .size_limit(1_048_576)
        .build();

    let regex = result.map_err(|e| match e {
        _ => Error::query("Invalid value for fuzzy match"),
    })?;

    match op {
        Exact => ok!(move |s| f(s) == exact),
        Fuzzy => ok!(move |s| regex.is_match(f(s))),
        _ => Err(Error::query("Invalid operation for text type")),
    }
}

fn intfn(f: IntFn, op: Operator, value: &str) -> Result<Filter> {
    let value: i64 = value.parse().map_err(|e| match e {
        _ => Error::query("Invalid value for number type"),
    })?;

    match op {
        Exact => ok!(move |s| f(s) == value),
        Fuzzy => ok!(move |s| f(s) == value),
        LessThan => ok!(move |s| f(s) < value),
        MoreThan => ok!(move |s| f(s) > value),
    }
}

fn dtufn(f: DtuFn, op: Operator, value: &str) -> Result<Filter> {
    let parsed = parse_date_string(value, Utc::now(), Dialect::Uk);

    let value: DateTime<Utc> = parsed.map_err(|e| match e {
        _ => Error::query("Invalid value for date type"),
    })?;

    let date = value.date();

    match op {
        Exact => ok!(move |s| match f(s) {
            Some(dt) => *dt == value,
            None => false,
        }),
        Fuzzy => ok!(move |s| match f(s) {
            Some(dt) => dt.date() == date,
            None => false,
        }),
        LessThan => ok!(move |s| match f(s) {
            Some(dt) => *dt < value,
            None => false,
        }),
        MoreThan => ok!(move |s| match f(s) {
            Some(dt) => *dt > value,
            None => false,
        }),
    }
}
