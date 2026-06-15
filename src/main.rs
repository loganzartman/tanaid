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

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args = Args::parse();

  let src = args
    .file_path
    .map_or(io::read_to_string(io::stdin()), fs::read_to_string)?;

  let parsed = parser::parse(src.as_str())?;
  println!("{:#?}", parsed);

  Ok(())
}

#[cfg(test)]
mod tests {
  #[test]
  fn greet_returns_hello() {}
}
