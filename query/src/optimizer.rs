//! Query optimizer.

use chrono::prelude::*;
use dateparser::parse_with_timezone;

use regex::escape;
use regex::RegexBuilder;

use fimfareader::archive::Story;
use fimfareader::error::Error;
use fimfareader::error::Result;

use super::parser::Operator;
use super::parser::Source;

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

    let Ok(regex) = result else {
        return Err(Error::query("Invalid value for fuzzy match"));
    };

    match op {
        Exact => ok!(move |s| f(s) == exact),
        Fuzzy => ok!(move |s| regex.is_match(f(s))),
        _ => Err(Error::query("Invalid operation for text type")),
    }
}

fn intfn(f: IntFn, op: Operator, value: &str) -> Result<Filter> {
    let Ok(value) = value.parse() else {
        return Err(Error::query("Invalid value for number type"));
    };

    match op {
        Exact => ok!(move |s| f(s) == value),
        Fuzzy => ok!(move |s| f(s) == value),
        LessThan => ok!(move |s| f(s) < value),
        MoreThan => ok!(move |s| f(s) > value),
    }
}

fn dtufn(f: DtuFn, op: Operator, value: &str) -> Result<Filter> {
    let Ok(value) = parse_with_timezone(value, &Local) else {
        return Err(Error::query("Invalid value for date type"));
    };

    match op {
        Exact => ok!(move |s| match f(s) {
            Some(dt) => *dt == value,
            None => false,
        }),
        Fuzzy => ok!(move |s| match f(s) {
            Some(dt) => dt.date_naive() == value.date_naive(),
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
