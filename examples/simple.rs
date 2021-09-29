use clap::{App, Arg};
use klask::Settings;

fn main() {
    let app = App::new("Example").arg(Arg::new("debug").short('d'));
    klask::run_app(app, Settings::default(), |matches| {
        println!("{}", matches.is_present("debug"))
    });
}
