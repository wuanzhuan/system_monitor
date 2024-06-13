use crate::event_list::Node;
use crate::event_record_model::EventRecordModel;
use crate::event_trace::EVENTS_DISPLAY_NAME_MAP;
use anyhow::{anyhow, Result};
use chumsky::prelude::*;
use once_cell::sync::Lazy;
use parking_lot::FairMutex;
use std::{collections::HashMap, sync::Arc};

static FILTER_EXPRESSION_FOR_ONE: Lazy<FairMutex<Option<ExpressionForOne>>> =
    Lazy::new(|| FairMutex::new(None));

static FILTER_EXPRESSION_FOR_PAIR: Lazy<FairMutex<Vec<ExpressionForPair>>> =
    Lazy::new(|| FairMutex::new(vec![]));

static CONTEXT_FOR_PAIR: Lazy<
    FairMutex<Vec<FairMutex<HashMap<String, Arc<Node<EventRecordModel>>>>>>,
> = Lazy::new(|| FairMutex::new(Vec::new()));

pub fn filter_for_one(
    fn_path_value: impl Fn(/*path*/ &Path, /*value*/ &Value) -> Result<bool> + Clone,
    fn_value: impl Fn(/*value*/ &Value) -> Result<bool> + Clone,
) -> Result<bool> {
    let lock = FILTER_EXPRESSION_FOR_ONE.lock();
    if let Some(ref expression) = *lock {
        expression.evaluate(fn_path_value, fn_value)
    } else {
        Ok(true)
    }
}

pub fn filter_for_pair(
    event_model_arc: &Arc<Node<EventRecordModel>>,
) -> Result<Option<Arc<Node<EventRecordModel>>>> {
    let lock = FILTER_EXPRESSION_FOR_PAIR.lock();

    for (index, expression_for_pair) in lock.iter().enumerate() {
        match expression_for_pair {
            ExpressionForPair::Handle => {
                match custom(
                    event_model_arc,
                    index,
                    "Object",
                    "CreateHandle",
                    "CloseHandle",
                    &[Path{key: String::from("process_id"), field: None}, Path{key: String::from("properties"), field: Some(String::from(""))}],
                ) {
                    Err(e) => return Err(e),
                    Ok((is_matched, node_arc)) => {
                        if is_matched {
                            return Ok(node_arc);
                        }
                    }
                }
            }
            ExpressionForPair::Memory => {
                match custom(
                    event_model_arc,
                    index,
                    "Memory",
                    "CreateHandle",
                    "CloseHandle",
                    &[Path{key: String::from("process_id"), field: None}, Path{key: String::from("properties"), field: Some(String::from(""))}],
                ) {
                    Err(e) => return Err(e),
                    Ok((is_matched, node_arc)) => {
                        if is_matched {
                            return Ok(node_arc);
                        }
                    }
                }
            }
            ExpressionForPair::Custom {
                event_name,
                opcode_name_first,
                opcode_name_second,
                path_for_match,
            } => {
                match custom(
                    event_model_arc,
                    index,
                    event_name,
                    opcode_name_first,
                    opcode_name_second,
                    path_for_match,
                ) {
                    Err(e) => return Err(e),
                    Ok((is_matched, node_arc)) => {
                        if is_matched {
                            return Ok(node_arc);
                        }
                    }
                }
            }
        }
    }
    return Ok(None);

    fn custom(
        event_model_arc: &Arc<Node<EventRecordModel>>,
        index: usize,
        event_name: &str,
        opcode_name_first: &str,
        opcode_name_second: &str,
        path_for_match: &[Path],
    ) -> Result<(
        /*is_matched*/ bool,
        Option<Arc<Node<EventRecordModel>>>,
    )> {
        if event_model_arc.value.array.event_name.to_ascii_lowercase()
            != event_name.to_ascii_lowercase()
        {
            return Ok((false, None));
        }
        let opcode_name = event_model_arc.value.array.opcode_name.to_ascii_lowercase();
        if opcode_name == opcode_name_first.to_ascii_lowercase() {
            match event_model_arc.value.get_key_by_paths(path_for_match) {
                Ok(key) => {
                    let vec_lock = CONTEXT_FOR_PAIR.lock();
                    let mut lock = vec_lock[index].lock();
                    lock.insert(key, event_model_arc.clone());
                    return Ok((true, None));
                }
                Err(e) => {
                    return Err(e);
                }
            }
        } else if opcode_name == opcode_name_second.to_ascii_lowercase() {
            match event_model_arc.value.get_key_by_paths(path_for_match) {
                Ok(key) => {
                    let vec_lock = CONTEXT_FOR_PAIR.lock();
                    let mut lock = vec_lock[index].lock();
                    let node_arc = lock.remove(&key);
                    return Ok((true, node_arc));
                }
                Err(e) => {
                    return Err(e);
                }
            }
        } else {
            Ok((false, None))
        }
    }
}

pub fn filter_expression_for_one_set(expression: Option<ExpressionForOne>) {
    *FILTER_EXPRESSION_FOR_ONE.lock() = expression;
}

pub fn filter_expression_for_pair_set(expressions: Vec<ExpressionForPair>) {
    *FILTER_EXPRESSION_FOR_PAIR.lock() = expressions;
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
        &self,
        fn_path_value: impl Fn(/*path*/ &Path, /*value*/ &Value) -> Result<bool> + Clone,
        fn_value: impl Fn(/*value*/ &Value) -> Result<bool> + Clone,
    ) -> Result<bool> {
        match self {
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

#[derive(Clone, Debug, PartialEq)]
pub enum ExpressionForPair {
    Handle,
    Memory,
    Custom {
        event_name: String,
        opcode_name_first: String,
        opcode_name_second: String,
        path_for_match: Vec<Path>,
    },
}

impl ExpressionForPair {
    pub fn parse(src: &str) -> Result<Vec<ExpressionForPair>> {
        match Self::build_parser().parse(src.trim()).into_result() {
            Err(e) => {
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
                Err(anyhow!(s))
            }

            Ok(ok) => {
                let mut err_string = String::with_capacity(100);
                for express in ok.iter() {
                    if let ExpressionForPair::Custom {
                        event_name,
                        opcode_name_first,
                        opcode_name_second,
                        path_for_match: fields_for_match,
                    } = express
                    {
                        if let Some(event_desc) =
                            EVENTS_DISPLAY_NAME_MAP.get(&event_name.to_ascii_lowercase())
                        {
                            if event_desc
                                .1
                                .get(&opcode_name_first.to_ascii_lowercase())
                                .is_none()
                            {
                                err_string.push_str(format!("No the opcode_name_first {opcode_name_first} for {event_name}\n").as_str());
                            }
                            if event_desc
                                .1
                                .get(&opcode_name_second.to_ascii_lowercase())
                                .is_none()
                            {
                                err_string.push_str(format!("No the opcode_name_first {opcode_name_second} for {event_name}\n").as_str());
                            }
                        } else {
                            err_string
                                .push_str(format!("No the event name {event_name}\n").as_str());
                        }
                        for path in fields_for_match.iter() {
                            if path.key.to_ascii_lowercase() != "properties" {
                                if path.field.is_some() {
                                    err_string.push_str(
                                        format!("The path {} no field\n", path.key).as_str(),
                                    );
                                }
                            } else {
                                if path.field.is_none() {
                                    err_string.push_str(
                                        format!("No specified field for properties\n").as_str(),
                                    );
                                }
                            }
                        }
                    }
                }
                if !err_string.is_empty() {
                    Err(anyhow!(err_string))
                } else {
                    let mut vec_lock = CONTEXT_FOR_PAIR.lock();
                    vec_lock.clear();
                    for _ in 0..ok.len() {
                        vec_lock.push(FairMutex::new(HashMap::new()));
                    }
                    Ok(ok)
                }
            }
        }
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
                    path_for_match: fields_for_match,
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
        assert_eq!(
            r,
            vec![
                ExpressionForPair::Handle,
                ExpressionForPair::Memory,
                ExpressionForPair::Custom {
                    event_name: "handle".to_string(),
                    opcode_name_first: "CreateHandle".to_string(),
                    opcode_name_second: "CloseHandle".to_string(),
                    path_for_match: vec![
                        Path {
                            key: "process_id".to_string(),
                            field: None,
                        },
                        Path {
                            key: "properties".to_string(),
                            field: Some("xx".to_string(),),
                        },
                    ],
                },
            ]
        );
    }
}
