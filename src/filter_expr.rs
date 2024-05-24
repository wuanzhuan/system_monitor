use anyhow::{Error, Result};
use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Invalid,
    Null,
    Bool(bool),
    Str(String),
    I64(i64),
    Num(f64),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    pub key: String,
    pub field: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FilterExpr {
    KeyWords,
    Parentheses(Box<FilterExpr>),
    Non(Box<FilterExpr>),
    And(Box<FilterExpr>, Box<FilterExpr>),
    Or(Box<FilterExpr>, Box<FilterExpr>),
    KvPair { key: Path, value: Value },
    FindValue(Value),
}

// start with #
pub enum KeyWords {
    HandlePair,
}

pub fn parse(src: &str) -> Result<FilterExpr> {
    parse_filter_expr()
        .parse(src.trim())
        .into_result()
        .map_err(|e| {
            let mut s = String::with_capacity(100);
            e.into_iter().for_each(|e| {
                let report = Report::build(ReportKind::Error, (), e.span().start)
                    .with_message(e.to_string())
                    .with_label(
                        Label::new(e.span().into_range())
                            .with_message(e.reason().to_string())
                            .with_color(Color::Red),
                    )
                    .finish();
                let mut vec = Vec::new();
                report.write(Source::from(&src), &mut vec).unwrap();
                s.push_str(format!("{}\n", String::from_utf8(vec).unwrap()).as_str());
            });
            Error::msg(s)
        })
}

fn parse_filter_expr<'a>() -> impl Parser<'a, &'a str, FilterExpr, extra::Err<Rich<'a, char>>> {
    recursive(|expr| {
        let value = recursive(|value| {
            let digits = text::digits(10).to_slice();

            let frac = just('.').then(digits);

            let exp = just('e')
                .or(just('E'))
                .then(one_of("+-").or_not())
                .then(digits);

            let number_i64 = just('-')
                .or_not()
                .then(text::int(10))
                .to_slice()
                .map(|s: &str| s.parse().unwrap())
                .boxed();

            let number = just('-')
                .or_not()
                .then(text::int(10))
                .then(frac.or_not())
                .then(exp.or_not())
                .to_slice()
                .map(|s: &str| s.parse().unwrap())
                .boxed();

            let escape = just('\\')
                .then(choice((
                    just('\\'),
                    just('/'),
                    just('"'),
                    just('b').to('\x08'),
                    just('f').to('\x0C'),
                    just('n').to('\n'),
                    just('r').to('\r'),
                    just('t').to('\t'),
                    just('u').ignore_then(text::digits(16).exactly(4).to_slice().validate(
                        |digits, e, emitter| {
                            char::from_u32(u32::from_str_radix(digits, 16).unwrap()).unwrap_or_else(
                                || {
                                    emitter
                                        .emit(Rich::custom(e.span(), "invalid unicode character"));
                                    '\u{FFFD}' // unicode replacement character
                                },
                            )
                        },
                    )),
                )))
                .ignored()
                .boxed();

            let string = none_of("\\\"")
                .ignored()
                .or(escape)
                .repeated()
                .to_slice()
                .map(ToString::to_string)
                .delimited_by(just('"'), just('"'))
                .boxed();

            let array = value
                .clone()
                .separated_by(just(',').padded().recover_with(skip_then_retry_until(
                    any().ignored(),
                    one_of(",]").ignored(),
                )))
                .allow_trailing()
                .collect()
                .padded()
                .delimited_by(
                    just('['),
                    just(']')
                        .ignored()
                        .recover_with(via_parser(end()))
                        .recover_with(skip_then_retry_until(any().ignored(), end())),
                )
                .boxed();

            let member = string.clone().then_ignore(just(':').padded()).then(value);
            let object = member
                .clone()
                .separated_by(just(',').padded().recover_with(skip_then_retry_until(
                    any().ignored(),
                    one_of(",}").ignored(),
                )))
                .collect()
                .padded()
                .delimited_by(
                    just('{'),
                    just('}')
                        .ignored()
                        .recover_with(via_parser(end()))
                        .recover_with(skip_then_retry_until(any().ignored(), end())),
                )
                .boxed();

            choice((
                just("null").to(Value::Null),
                just("true").to(Value::Bool(true)),
                just("false").to(Value::Bool(false)),
                number_i64.map(Value::I64),
                number.map(Value::Num),
                string.map(Value::Str),
                array.map(Value::Array),
                object.map(Value::Object),
            ))
            .recover_with(via_parser(nested_delimiters(
                '{',
                '}',
                [('[', ']')],
                |_| Value::Invalid,
            )))
            .recover_with(via_parser(nested_delimiters(
                '[',
                ']',
                [('{', '}')],
                |_| Value::Invalid,
            )))
            .recover_with(skip_then_retry_until(
                any().ignored(),
                one_of(",]}").ignored(),
            ))
            .padded()
        });

        let path = text::ident()
            .then(just(".").ignore_then(text::ident()).or_not())
            .map(|(key, field): (&str, Option<&str>)| Path {
                key: key.to_string(),
                field: field.map(|s| String::from(s)),
            });
        let kv_pair = path
            .then_ignore(just("=").padded())
            .then(value)
            .map(|(key, value)| FilterExpr::KvPair { key, value })
            .boxed();
        let parentheses = just("(")
            .padded()
            .ignore_then(expr.clone())
            .then_ignore(just(")").padded())
            .map(|expr| FilterExpr::Parentheses(Box::new(expr)))
            .boxed();
        let op_non = just("!")
            .padded()
            .ignore_then(expr.clone())
            .map(|expr| FilterExpr::Non(Box::new(expr)))
            .boxed();
        let start = choice((parentheses, op_non, kv_pair));

        let op = choice((
            just("&&").padded().to(FilterExpr::And as fn(_, _) -> _),
            just("||").padded().to(FilterExpr::Or as fn(_, _) -> _),
        ));

        start.foldl(op.then(expr.clone()).repeated(), |a, (op, rhs)| {
            op(Box::new(a), Box::new(rhs))
        })
    })
    .then_ignore(end())
}

#[cfg(test)]
mod tests {
    use super::{FilterExpr, Path, Value};
    #[test]
    fn success() {
        let src = r#"(key1.field = 1.556) && key2 = 2.55"#;
        //let (json, errs) = parse_test().parse(src.trim()).into_output_errors();
        let r = super::parse(src.trim());
        assert_eq!(
            r.unwrap(),
            FilterExpr::And(
                Box::new(FilterExpr::Parentheses(Box::new(FilterExpr::KvPair {
                    key: Path {
                        key: "key1".to_string(),
                        field: Some("field".to_string(),),
                    },
                    value: Value::Num(1.556,),
                }),)),
                Box::new(FilterExpr::KvPair {
                    key: Path {
                        key: "key2".to_string(),
                        field: None,
                    },
                    value: Value::Num(2.55,),
                }),
            ),
        );
    }

    #[test]
    fn fail() {
        let src = r#"(key1.field = 1.556 && key2 = 2.55"#;
        let r = super::parse(src.trim());
        assert!(r.is_err());
        println!("{}", r.err().unwrap());
    }

    #[test]
    fn value_i64() {
        let src = r#"key = 2555555554421"#;
        let r = super::parse(src.trim()).unwrap();
        assert_eq!(
            r,
            FilterExpr::KvPair {
                key: Path {
                    key: "key".to_string(),
                    field: None,
                },
                value: Value::I64(2555555554421,),
            }
        );
    }
}
