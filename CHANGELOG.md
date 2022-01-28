## Version X.X.X
- Added localization settings
- Added style settings, for setting egui styling
- Added `#[non_exhaustive]` to setting so adding new ones won't be a breaking change

## Version 1.0.0
- Update `clap` to `3.0`!
- Add support for custom fonts

## Version 0.4.0
- Update `clap` to `-beta.5`
- Update `eframe` to `0.15.0`

## Version 0.3.1
- Pin `clap` version to "=3.0.0-beta.4"

## Version 0.3.0
- Optionally allow setting environment variables, stdin and working directory
- Progress bars in the output!
- You can now copy output
- Internal improvements

## Version 0.2.3
- Klask doesn't require nightly to compile!
- Much better command generation. Klask should now correctly handle arguments with any combination of: multiple values, requiring equals, requiring delimiters (currently only the default ',').
- Removed unnecessary features, so they won't be compiled if they aren't needed.
- Internal improvements

## Version 0.2.2
- Improve visuals. Now arguments are aligned.