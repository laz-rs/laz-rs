# laz-rs

Implementation or rather, translation of LAZ (laszip compression) format in Rust.

The goal of this project is to be a port of the Laszip compression,
allowing LAS readers to be able to read / write LAZ data, but not a fully featured LAS reader.

If you want a user friendly Rust LAS reader, [las-rs](https://crates.io/crates/las) is what you 
are looking for. `las-rs` can use `laz-rs` to manage LAZ data by enabling the `laz` 
feature of the `las-rs` crate.

Original Implementations:
 - [LASzip](https://github.com/LASzip/LASzip)
 - [LAStools](https://github.com/LAStools/LAStools)
 - [laz-perf](https://github.com/hobu/laz-perf)
 
 
Minimal Rust version: 1.40.0