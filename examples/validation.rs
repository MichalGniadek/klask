//! Value validation
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use clap::Parser;
use klask::Settings;

#[derive(Debug, Parser)]
#[clap(name = "Validation Example")]
/// Help is displayed at the top
pub struct Validation {
    #[clap(long, value_parser(is_hello))]
    input: String,
}

fn is_hello(input: &str) -> Result<String, String> {
    match input {
        "hello" => Ok(input.to_string()),
        _ => Err("Is not \"hello\"".to_string()),
    }
}

fn main() {
    klask::run_derived::<Validation, _>(Settings::default(), |o| println!("{:#?}", o));
}
