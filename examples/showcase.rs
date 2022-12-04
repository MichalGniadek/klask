//! Showcases clap parsing and different widgets
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use clap::{Parser, ValueHint};
use klask::Settings;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[clap(name = "App name")]
/// Help is displayed at the top
pub struct Showcase {
    /// Argument help is displayed as tooltips
    required_field: String,
    #[clap(long)]
    optional_field: Option<String>,
    #[clap(long, default_value = "default value")]
    field_with_default: String,
    #[clap(long)]
    flag: bool,
    #[clap(short, parse(from_occurrences))]
    count_occurrences_as_a_nice_counter: i32,
    #[clap(subcommand)]
    subcommand: Subcommand,
}

#[derive(Debug, Parser)]
pub enum Subcommand {
    /// Subcommands also display help
    SubcommandA {
        #[clap(long, parse(from_os_str), value_hint = ValueHint::AnyPath)]
        native_path_picker: Option<PathBuf>,
        #[clap(possible_values = &["One", "Two", "Three"])]
        choose_one: String,
        #[clap(subcommand)]
        inner: InnerSubcommand,
    },
    SubcommandB {},
}

#[derive(Debug, Parser)]
pub enum InnerSubcommand {
    InnerSubcommandA {
        #[clap(short, multiple_occurrences(true), default_value = "initial value")]
        multiple_values: Vec<String>,
    },
    /// About
    InnerSubcommandB {
        #[clap(subcommand)]
        inner: InnerInnerSubcommand,
    },
    InnerSubcommandC,
    InnerSubcommandD,
}

#[derive(Debug, Parser)]
pub enum InnerInnerSubcommand {
    /// About 2
    A,
    B,
}

fn main() {
    klask::run_derived::<Showcase, _>(Settings::default(), |o| println!("{:#?}", o));
}
