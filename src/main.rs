use clap::Parser;
use std::{
  fs,
  io::{self},
};

mod parser;

#[derive(Parser, Debug)]
struct Args {
  file_path: Option<String>,
}

fn main() {
  let args = Args::parse();

  let src = args
    .file_path
    .map_or(io::read_to_string(io::stdin()), |path| {
      fs::read_to_string(path)
    })
    .expect("Failed to read file");

  let parsed = parser::parse(src.as_str());
  println!("{}", parsed);
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn greet_returns_hello() {}
}
