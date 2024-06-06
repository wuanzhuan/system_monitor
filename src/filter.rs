use anyhow::{anyhow, Result};
use chumsky::prelude::*;
use once_cell::sync::Lazy;
use parking_lot::FairMutex;
use std::{collections::HashMap, sync::Arc};

static FILTER_EXPRESSION: Lazy<
    FairMutex<(
        Arc<Option<ExpressionForOne>>,
        Arc<Vec<ExpressionForPair>>,
    )>,
> = Lazy::new(|| FairMutex::new((Arc::new(None), Arc::new(vec![]))));

pub fn filter_expression_for_one_set(expression: Option<ExpressionForOne>) {
    let mut lock = FILTER_EXPRESSION.lock();
    lock.0 = Arc::new(expression);
}

pub fn filter_expression_for_pair_set(expressions: Vec<ExpressionForPair>) {
    let mut lock = FILTER_EXPRESSION.lock();
    lock.1 = Arc::new(expressions);
}

pub fn filter_expression_get() -> (
    Arc<Option<ExpressionForOne>>,
    Arc<Vec<ExpressionForPair>>,
) {
    let lock = FILTER_EXPRESSION.lock();
    (lock.0.clone(), lock.1.clone())
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExpressionForOne {
    Parentheses(Box<ExpressionForOne>),
    Non(Box<ExpressionForOne>),
    And(Box<ExpressionForOne>, Box<ExpressionForOne>),
    Or(Box<ExpressionForOne>, Box<ExpressionForOne>),
    KvPair { key: Path, value: Value },
    FindValue(Value),
}

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

impl ExpressionForOne {
    pub fn parse(src: &str) -> Result<ExpressionForOne> {
        Self::build_parser()
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
        expr: &ExpressionForOne,
        fn_path_value: impl Fn(/*path*/ &Path, /*value*/ &Value) -> Result<bool> + Clone,
        fn_value: impl Fn(/*value*/ &Value) -> Result<bool> + Clone,
    ) -> Result<bool> {
        match expr {
            ExpressionForOne::Parentheses(expr) => {
                return ExpressionForOne::evaluate(expr, fn_path_value, fn_value)
            }
            ExpressionForOne::Non(expr) => {
                return ExpressionForOne::evaluate(expr, fn_path_value, fn_value).map(|ok| !ok)
            }
            ExpressionForOne::And(expr_left, expr_right) => {
                return Ok(ExpressionForOne::evaluate(
                    expr_left,
                    fn_path_value.clone(),
                    fn_value.clone(),
                )? && ExpressionForOne::evaluate(expr_right, fn_path_value, fn_value)?);
            }
            ExpressionForOne::Or(expr_left, expr_right) => {
                return Ok(ExpressionForOne::evaluate(
                    expr_left,
                    fn_path_value.clone(),
                    fn_value.clone(),
                )? || ExpressionForOne::evaluate(expr_right, fn_path_value, fn_value)?);
            }
            ExpressionForOne::KvPair { key, value } => {
                return fn_path_value(key, value);
            }
            ExpressionForOne::FindValue(value) => {
                return fn_value(value);
            }
        }
    }

    fn build_parser<'a>() -> impl Parser<'a, &'a str, ExpressionForOne, extra::Err<Rich<'a, char>>>
    {
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
                                char::from_u32(u32::from_str_radix(digits, 16).unwrap())
                                    .unwrap_or_else(|| {
                                        emitter.emit(Rich::custom(
                                            e.span(),
                                            "invalid unicode character",
                                        ));
                                        '\u{FFFD}' // unicode replacement character
                                    })
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
            let parentheses = just("(")
                .padded()
                .ignore_then(expr.clone())
                .then_ignore(just(")").padded())
                .map(|expr| ExpressionForOne::Parentheses(Box::new(expr)))
                .boxed();
            let op_non = just("!")
                .padded()
                .ignore_then(expr.clone())
                .map(|expr| ExpressionForOne::Non(Box::new(expr)))
                .boxed();
            let kv_pair = path
                .then_ignore(just("=").padded())
                .then(value.clone())
                .map(|(key, value)| ExpressionForOne::KvPair { key, value })
                .boxed();
            let find_value = value.map(ExpressionForOne::FindValue);

            let op_logic = choice((
                just("&&")
                    .padded()
                    .to(ExpressionForOne::And as fn(_, _) -> _),
                just("||")
                    .padded()
                    .to(ExpressionForOne::Or as fn(_, _) -> _),
            ));
            let start = choice((parentheses, op_non, kv_pair, find_value));

            start.foldl(op_logic.then(expr.clone()).repeated(), |a, (op, rhs)| {
                op(Box::new(a), Box::new(rhs))
            })
        })
        .then_ignore(end())
    }
}

// start with #
#[derive(Clone, Debug, PartialEq)]
pub enum ExpressionForPair {
    Handle,
    Memory,
    Custom {
        event_name: String,
        opcode_name_first: String,
        opcode_name_second: String,
        fields_for_match: Vec<Path>,
    },
}

impl ExpressionForPair {
    pub fn parse(src: &str) -> Result<Vec<ExpressionForPair>> {
        Self::build_parser()
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

    fn build_parser<'a>(
    ) -> impl Parser<'a, &'a str, Vec<ExpressionForPair>, extra::Err<Rich<'a, char>>> {
        let path = text::ident()
            .then(just(".").ignore_then(text::ident()).or_not())
            .map(|(key, field): (&str, Option<&str>)| Path {
                key: key.to_string(),
                field: field.map(|s| String::from(s)),
            });
        let event_opcode_names = text::ascii::ident()
            .map(|s: &str| s.to_string())
            .then_ignore(just(",").padded())
            .repeated()
            .exactly(3)
            .collect::<Vec<String>>();
        let fields_for_match = path
            .separated_by(just(",").padded())
            .at_least(1)
            .collect::<Vec<Path>>();
        let custom_parameters = just("(")
            .padded()
            .ignore_then(event_opcode_names.then(fields_for_match))
            .then_ignore(just(")").padded());
        let expression_for_pair = choice((
            just("handle").to(ExpressionForPair::Handle),
            just("memory").to(ExpressionForPair::Memory),
            just("custom").ignore_then(custom_parameters).map(
                |(event_opcode_names, fields_for_match)| ExpressionForPair::Custom {
                    event_name: event_opcode_names[0].clone(),
                    opcode_name_first: event_opcode_names[1].clone(),
                    opcode_name_second: event_opcode_names[2].clone(),
                    fields_for_match: fields_for_match,
                },
            ),
        ));

        expression_for_pair
            .separated_by(just("||").padded())
            .collect::<Vec<ExpressionForPair>>()
            .then_ignore(end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success() {
        let src = r#"(key1.field = 1.556) && key2 = 2.55"#;
        //let (json, errs) = parse_test().parse(src.trim()).into_output_errors();
        let r = ExpressionForOne::parse(src.trim());
        assert_eq!(
            r.unwrap(),
            ExpressionForOne::And(
                Box::new(ExpressionForOne::Parentheses(Box::new(
                    ExpressionForOne::KvPair {
                        key: Path {
                            key: "key1".to_string(),
                            field: Some("field".to_string(),),
                        },
                        value: Value::Num(1.556,),
                    }
                ),)),
                Box::new(ExpressionForOne::KvPair {
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
        let r = ExpressionForOne::parse(src.trim());
        assert!(r.is_err());
        println!("{}", r.err().unwrap());
    }

    #[test]
    fn value_i64() {
        let src = r#"key = 2555555554421"#;
        let r = ExpressionForOne::parse(src.trim()).unwrap();
        assert_eq!(
            r,
            ExpressionForOne::KvPair {
                key: Path {
                    key: "key".to_string(),
                    field: None,
                },
                value: Value::I64(2555555554421,),
            }
        );
    }

    #[test]
    fn expression_for_pair_succuss() {
        let src = r#"handle || memory"#;
        let r = ExpressionForPair::parse(src.trim()).unwrap();
        assert_eq!(
            r,
            vec![ExpressionForPair::Handle, ExpressionForPair::Memory]
        );
    }

    #[test]
    fn expression_for_pair_custom_succuss() {
        let src = r#"handle || memory || custom(handle, CreateHandle, CloseHandle, process_id, properties.xx)"#;
        let r = ExpressionForPair::parse(src.trim()).unwrap();
        println!("xxx: {r:?}");
    }
}
