use anyhow::{Result, anyhow};
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
    KeyWords(KeyWords),
    Parentheses(Box<FilterExpr>),
    Non(Box<FilterExpr>),
    And(Box<FilterExpr>, Box<FilterExpr>),
    Or(Box<FilterExpr>, Box<FilterExpr>),
    KvPair { key: Path, value: Value },
    FindValue(Value),
}

// start with #
#[derive(Clone, Debug, PartialEq)]
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
                s.push_str(
                    format!(
                        "Error happens at the {}th letter: {}\n",
                        e.span().start,
                        e.to_string()
                    )
                    .as_str(),
                );
            });
            anyhow!(s)
        })
}

pub fn evaluate(
    expr: &FilterExpr,
    fn_path_value: impl Fn(/*path*/ &Path, /*value*/ &Value) -> Result<bool> + Clone,
    fn_value: impl Fn(/*value*/ &Value) -> Result<bool> + Clone,
) -> Result<bool> {
    match expr {
        FilterExpr::KeyWords(_key_words) => { return Err(anyhow!("Not supported key words find now"))}
        FilterExpr::Parentheses(expr) => { return evaluate(expr, fn_path_value, fn_value) }
        FilterExpr::Non(expr) => { return evaluate(expr, fn_path_value, fn_value).map(| ok| !ok) }
        FilterExpr::And(expr_left, expr_right) => {
            return Ok(evaluate(expr_left, fn_path_value.clone(), fn_value.clone())? && evaluate(expr_right, fn_path_value, fn_value)?);
        }
        FilterExpr::Or(expr_left, expr_right) => {
            return Ok(evaluate(expr_left, fn_path_value.clone(), fn_value.clone())? || evaluate(expr_right, fn_path_value, fn_value)?);
        }
        FilterExpr::KvPair{key, value} => {
            return fn_path_value(key, value);
        }
        FilterExpr::FindValue(value) => {
            return fn_value(value);
        }
    }
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

        let keywords = just("#")
            .ignore_then(choice((
                just("handle_pair").to(FilterExpr::KeyWords(KeyWords::HandlePair)),
            )))
            .padded();
        let path = text::ident()
            .then(just(".").ignore_then(text::ident()).or_not())
            .map(|(key, field): (&str, Option<&str>)| Path {
                key: key.to_string(),
                field: field.map(|s| String::from(s)),
            });
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
        let kv_pair = path
            .then_ignore(just("=").padded())
            .then(value.clone())
            .map(|(key, value)| FilterExpr::KvPair { key, value })
            .boxed();
        let find_value = value.map(FilterExpr::FindValue);

        let op_logic = choice((
            just("&&").padded().to(FilterExpr::And as fn(_, _) -> _),
            just("||").padded().to(FilterExpr::Or as fn(_, _) -> _),
        ));
        let start = choice((keywords, parentheses, op_non, kv_pair, find_value));

        start.foldl(op_logic.then(expr.clone()).repeated(), |a, (op, rhs)| {
            op(Box::new(a), Box::new(rhs))
        })
    })
    .then_ignore(end())
}

#[cfg(test)]
mod tests {
    use super::{FilterExpr, KeyWords, Path, Value};

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

    #[test]
    fn keywords_handlepair() {
        let src = r"#handle_pair";
        let r = super::parse(src.trim()).unwrap();
        assert_eq!(r, FilterExpr::KeyWords(KeyWords::HandlePair));
    }
}
