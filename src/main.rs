#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use klask::Klask;
use std::{thread, time};

fn main() {
    Klask::run_derived::<klask::example_opts::Opts, _>(|o| {
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
