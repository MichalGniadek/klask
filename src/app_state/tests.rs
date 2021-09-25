use super::AppState;
use crate::arg_state::ArgState;
use clap::{Clap, FromArgMatches, IntoApp};
use std::fmt::Debug;

#[derive(Debug, Clap, PartialEq, Eq)]
struct OptionalAndDefault {
    required: String,
    optional: Option<String>,
    #[clap(default_value = "d")]
    default: String,
}

#[test]
fn check_optional_and_default() {
    test_app(
        |args| args[0].enter_value("a"),
        OptionalAndDefault {
            required: "a".into(),
            optional: None,
            default: "d".into(),
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
    let matches = app.try_get_matches_from(args.iter()).unwrap();
    let c = C::from_arg_matches(&matches).unwrap();
    assert_eq!(c, expected, "with args: {:?}", &args[1..]);
}

impl crate::arg_state::ArgState {
    fn enter_value(&mut self, val: &str) {
        use crate::arg_state::{ArgKind, ChooseState};
        match &mut self.kind {
            ArgKind::String { value, .. } => *value = val.to_string(),
            ArgKind::Path { value, .. } => *value = val.to_string(),
            ArgKind::Choose {
                value: ChooseState(value, _),
                ..
            } => *value = val.to_string(),
            _ => panic!("Called enter_value on {:?}", self),
        }
    }
}
