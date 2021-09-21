#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use clap::Clap;
use klask::Klask;
use std::{thread, time};

/// This doc string acts as a help message when the user runs '--help'
/// as do all doc strings on fields
#[derive(Debug, Clap)]
#[clap(version = "1.0", author = "author", name = "name")]
pub struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
    /// Sets a custom config file. Could have been an Option<T> with no default too
    #[clap(short, long, default_value = "default.conf")]
    config: String,
    /// Some input. Because this isn't an Option<T> it's required to be used
    input: String,
    /// A level of verbosity, and can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
}

#[derive(Debug, Clap)]
pub enum SubCommand {
    #[clap(version = "1.3", author = "Someone E. <someone_else@other.com>")]
    Test(Test),
    Run(Run),
    Walk(Walk),
}

/// A subcommand for controlling testing
#[derive(Debug, Clap)]
pub struct Test {
    /// Print debug info
    #[clap(short)]
    debug: bool,
}

/// A subcommand for running
#[derive(Debug, Clap)]
pub struct Run {
    /// Example
    lalala: String,
    hoho: Option<i32>,
}

/// A subcommand for running2
#[derive(Debug, Clap)]
pub struct Walk {
    /// Example2
    #[clap(multiple_occurrences(true))]
    mult: Vec<String>,
}

fn main() {
    Klask::run_derived::<Opts, _>(|o| {
        println!("{:#?}", o);
        loop {
            thread::sleep(time::Duration::from_secs(1));
            println!("A");
        }
    });
    // Klask::run_app(
    //     clap::App::new("Name").arg(clap::Arg::new("test").short('t').default_value("def")),
    //     |m| println!("{:#?}", m),
    // );
}
