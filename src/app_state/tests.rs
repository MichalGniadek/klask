use super::AppState;
use crate::arg_state::{ArgKind, ArgState, ChooseState};
use clap::{Clap, FromArgMatches, IntoApp, ValueHint};
use std::{fmt::Debug, path::PathBuf};
use uuid::Uuid;

#[derive(Debug, Clap, PartialEq, Eq)]
struct ForbidEmpty {
    #[clap(long, forbid_empty_values = true)]
    optional_no_empty1: Option<String>,
    #[clap(long, forbid_empty_values = true)]
    optional_no_empty2: Option<String>,
    #[clap(long, forbid_empty_values = true)]
    optional_no_empty3: Option<String>,
}

#[test]
fn forbid_empty() {
    test_app(
        |args| {
            args[0].enter_value("a");
            args[2].enter_value("");
        },
        ForbidEmpty {
            optional_no_empty1: Some("a".into()),
            optional_no_empty2: None,
            optional_no_empty3: None,
        },
    );
}

#[derive(Debug, Clap, PartialEq, Eq)]
struct OptionalAndDefault {
    required: String,
    optional: Option<String>,
    #[clap(default_value = "d")]
    default: String,
}

#[test]
fn optional_and_default() {
    test_app(
        |args| args[0].enter_value("a"),
        OptionalAndDefault {
            required: "a".into(),
            optional: None,
            default: "d".into(),
        },
    );
}

#[derive(Debug, Clap, PartialEq, Eq)]
struct UseEquals {
    #[clap(long, require_equals = true)]
    long: String,
    #[clap(short, require_equals = true)]
    short: String,
    #[clap(long, require_equals = true, value_hint = ValueHint::AnyPath)]
    path: PathBuf,
    #[clap(long, require_equals = true, possible_values = &["P", "O"])]
    choose: String,
    #[clap(long, require_equals = true, multiple_occurrences = true)]
    multiple: Vec<String>,
}

#[test]
fn use_equals() {
    test_app(
        |args| {
            args[0].enter_value("a");
            args[1].enter_value("b");
            args[2].enter_value("c");
            args[3].enter_value("P");
            args[4].enter_values(&["d", "e"]);
        },
        UseEquals {
            long: "a".into(),
            short: "b".into(),
            path: "c".into(),
            choose: "P".into(),
            multiple: vec!["d".into(), "e".into()],
        },
    );
}

fn test_app<C, F>(setup: F, expected: C)
where
    C: IntoApp + FromArgMatches + Debug + Eq,
    F: FnOnce(&mut Vec<ArgState>),
{
    let app = C::into_app();
    let mut app_state = AppState::new(&app);
    setup(&mut app_state.args);
    let args = app_state.get_cmd_args(vec!["_name".into()]).unwrap();
    eprintln!("Args: {:?}", &args[1..]);
    let matches = app.try_get_matches_from(args.iter()).unwrap();
    let c = C::from_arg_matches(&matches).unwrap();
    assert_eq!(c, expected);
}

impl crate::arg_state::ArgState {
    fn enter_value(&mut self, val: &str) {
        match &mut self.kind {
            ArgKind::String { value, .. }
            | ArgKind::Path { value, .. }
            | ArgKind::Choose {
                value: ChooseState(value, _),
                ..
            } => *value = val.to_string(),
            _ => panic!("Called enter_value on {:?}", self),
        }
    }

    fn enter_values(&mut self, vals: &[&str]) {
        match &mut self.kind {
            ArgKind::MultipleStrings { values, .. } | ArgKind::MultiplePaths { values, .. } => {
                *values = vals.iter().map(|s| s.to_string()).collect()
            }
            ArgKind::MultipleChoose { values, .. } => {
                *values = vals
                    .iter()
                    .map(|s| ChooseState(s.to_string(), Uuid::default()))
                    .collect()
            }
            _ => panic!("Called enter_value on {:?}", self),
        }
    }
}
