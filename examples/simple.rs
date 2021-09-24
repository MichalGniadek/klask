use clap::{App, Arg};

fn main() {
    let app = App::new("Example").arg(Arg::new("debug").short('d'));
    klask::run_app(app, |matches| println!("{}", matches.is_present("debug")));
}
