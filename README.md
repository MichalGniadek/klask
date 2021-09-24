[![crates.io](https://img.shields.io/crates/v/klask?style=for-the-badge)](https://crates.io/crates/klask)
[![license](https://img.shields.io/crates/l/klask?style=for-the-badge)](LICENSE)
[![docs.rs](https://img.shields.io/docsrs/klask?style=for-the-badge)](https://docs.rs/klask)

# Klask
Allows you to create a gui application automatically from clap (v3). Uses egui for graphics. Currently, requires nightly.

## Features
- Supports optional fields with and without default values
- Supports flags with multiple occurrences (`-vvv`)
- Has a native path picker
- Supports fields with multiple values
- Output is colored and has clickable links

Unfortunately there are still many edge- (and not so edge-) cases where the command generation breaks.

Example gui:

![image showcasing the gui](image0.png)

Generated from miniserve's app:

![image showcasing the gui](image1.png)
