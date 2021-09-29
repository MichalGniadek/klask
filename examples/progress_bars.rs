use clap::App;
use klask::Settings;
use std::thread;
use std::time::Duration;

fn main() {
    klask::run_app(App::new("Progress bars"), Settings::default(), |_| {
        const MAX: u64 = 100;

        for i in 0..=MAX {
            // You must pass in a value between [0, 1]
            klask::progress_bar("Static description", i as f32 / MAX as f32);
            klask::progress_bar_with_id(
                "Progress", // has to be a hashable id that identifies this progress bar
                &format!("Dynamic description [{}/{}]", i, MAX),
                i as f32 / MAX as f32,
            );

            thread::sleep(Duration::from_millis(20));
        }

        println!("Finished!")
    });
}
