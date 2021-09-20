use clap::Clap;
use klask::example_opts::Opts;

fn main() {
    let o = Opts::parse();
    println!("{:#?}", o);
}
