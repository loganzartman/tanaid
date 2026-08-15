//! Runs every script in `sample/` under both the `tanaid` binary and `tclsh`,
//! and fails when their stdout or success status disagree. `tclsh` is the
//! expected output; nothing is committed as a golden file.
//!
//! Adding a `.tcl` file to `sample/` adds a test case. Two optional sibling
//! files change how a case runs:
//!
//! - `sample/<name>.stdin` is piped to both interpreters' stdin.
//! - `sample/<name>.xfail` marks a known divergence. Its contents are the
//!   reason. The case then passes when the outputs differ, and fails when they
//!   match, so the marker cannot go stale unnoticed.
//!
//! Set `TCLSH` to use a specific interpreter instead of the one on `PATH`.

use libtest_mimic::{Arguments, Failed, Trial};
use std::env;
use std::fmt::Write as _;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Output, Stdio};

// Relative to the manifest, not the cwd: `cargo test` runs from the package
// root, but sample/ lives at the workspace root.
const SAMPLE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../sample");

// Cargo builds the bin and sets this for integration tests of the crate that
// declares it. The bin is gated on the `cli` feature, which is on by default.
const TANAID: &str = env!("CARGO_BIN_EXE_tanaid");

fn main() -> ExitCode {
  let args = Arguments::from_args();

  let tclsh = match resolve_tclsh() {
    Ok(tclsh) => tclsh,
    Err(err) => {
      eprintln!("{err}");
      return ExitCode::FAILURE;
    }
  };

  let trials = match collect_trials(&tclsh) {
    Ok(trials) => trials,
    Err(err) => {
      eprintln!("{err}");
      return ExitCode::FAILURE;
    }
  };

  libtest_mimic::run(&args, trials).exit()
}

/// Finds the reference interpreter and checks that it can actually be run.
/// A missing tclsh fails the whole suite rather than silently skipping it.
fn resolve_tclsh() -> Result<PathBuf, String> {
  let tclsh = env::var_os("TCLSH")
    .map(PathBuf::from)
    .unwrap_or_else(|| PathBuf::from("tclsh"));

  // With no script argument and no stdin, tclsh reads EOF and exits. This only
  // checks that the program exists and starts.
  let probe = Command::new(&tclsh)
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .status();

  match probe {
    Ok(_) => Ok(tclsh),
    Err(err) => Err(format!(
      "cannot run tclsh at `{}`: {err}\n\
       The sample suite compares tanaid against a real Tcl interpreter. Install tclsh, \
       or set TCLSH to the path of one.",
      tclsh.display()
    )),
  }
}

fn collect_trials(tclsh: &Path) -> Result<Vec<Trial>, String> {
  // SAMPLE_DIR reaches the workspace root by going up out of the package dir;
  // canonicalizing keeps the `../..` out of every path we print.
  let dir = fs::canonicalize(SAMPLE_DIR)
    .map_err(|err| format!("cannot read {SAMPLE_DIR}: {err}"))?;
  let entries =
    fs::read_dir(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;

  let mut scripts = Vec::new();
  for entry in entries {
    let path = entry
      .map_err(|err| format!("cannot read {}: {err}", dir.display()))?
      .path();
    if path.extension().is_some_and(|ext| ext == "tcl") {
      let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| format!("sample name is not valid UTF-8: {}", path.display()))?
        .to_owned();
      scripts.push((name, path));
    }
  }
  // read_dir order is platform-dependent; the test list should not be.
  scripts.sort();

  Ok(
    scripts
      .into_iter()
      .map(|(name, script)| {
        let case = Case {
          tclsh: tclsh.to_owned(),
          stdin: sibling(&script, "stdin"),
          xfail: sibling(&script, "xfail"),
          script,
        };
        Trial::test(name, move || run_case(&case))
      })
      .collect(),
  )
}

/// The path `sample/<name>.<ext>`, if that file exists.
fn sibling(script: &Path, ext: &str) -> Option<PathBuf> {
  let path = script.with_extension(ext);
  path.is_file().then_some(path)
}

struct Case {
  script: PathBuf,
  tclsh: PathBuf,
  stdin: Option<PathBuf>,
  xfail: Option<PathBuf>,
}

fn run_case(case: &Case) -> Result<(), Failed> {
  let tanaid = run_interpreter(Path::new(TANAID), case)?;
  let tclsh = run_interpreter(&case.tclsh, case)?;
  let mismatch = describe_mismatch(&tclsh, &tanaid);

  match (mismatch, &case.xfail) {
    (None, None) => Ok(()),
    (Some(mismatch), None) => Err(mismatch.into()),
    (Some(_), Some(_)) => Ok(()),
    (None, Some(xfail)) => {
      let reason = fs::read_to_string(xfail)
        .map_err(|err| format!("cannot read {}: {err}", xfail.display()))?;
      Err(
        format!(
          "tanaid now matches tclsh, but {} marks this as a known divergence:\n  {}\n\
           Delete that file.",
          xfail.display(),
          reason.trim()
        )
        .into(),
      )
    }
  }
}

fn run_interpreter(program: &Path, case: &Case) -> Result<Output, Failed> {
  let stdin = match &case.stdin {
    Some(path) => Stdio::from(
      File::open(path).map_err(|err| format!("cannot open {}: {err}", path.display()))?,
    ),
    // Also keeps tanaid out of its REPL branch, which triggers on a tty stdin.
    None => Stdio::null(),
  };

  Command::new(program)
    .arg(&case.script)
    // sample/ lives at the workspace root; run scripts from there so any
    // relative path a sample uses resolves the same way for both interpreters.
    .current_dir(Path::new(SAMPLE_DIR).parent().expect("sample dir has a parent"))
    .stdin(stdin)
    .output()
    .map_err(|err| format!("cannot run {}: {err}", program.display()).into())
}

/// `None` when the two runs agree. Compares stdout exactly, and success rather
/// than the exact exit code: tanaid only ever returns 0 or 1, while Tcl uses
/// other codes. stderr is not compared — tanaid's error wording is its own.
fn describe_mismatch(tclsh: &Output, tanaid: &Output) -> Option<String> {
  if tclsh.stdout == tanaid.stdout && tclsh.status.success() == tanaid.status.success() {
    return None;
  }

  let tclsh_stdout = String::from_utf8_lossy(&tclsh.stdout);
  let tanaid_stdout = String::from_utf8_lossy(&tanaid.stdout);

  let mut message = String::new();
  writeln!(message, "tclsh exited with {}", tclsh.status).unwrap();
  writeln!(message, "tanaid exited with {}", tanaid.status).unwrap();

  if tclsh.stdout != tanaid.stdout {
    writeln!(message, "\nstdout diff (- tclsh, + tanaid):").unwrap();
    message.push_str(&line_diff(
      &tclsh_stdout.lines().collect::<Vec<_>>(),
      &tanaid_stdout.lines().collect::<Vec<_>>(),
    ));
  }

  for (name, output) in [("tclsh", tclsh), ("tanaid", tanaid)] {
    if !output.status.success() && !output.stderr.is_empty() {
      writeln!(
        message,
        "\n{name} stderr:\n{}",
        String::from_utf8_lossy(&output.stderr).trim_end()
      )
      .unwrap();
    }
  }

  Some(message)
}

/// A diff over the longest common subsequence of lines. Sample outputs are tens
/// of lines, so the quadratic table is not worth avoiding.
fn line_diff(expected: &[&str], actual: &[&str]) -> String {
  let (n, m) = (expected.len(), actual.len());
  let mut lcs = vec![vec![0usize; m + 1]; n + 1];
  for i in (0..n).rev() {
    for j in (0..m).rev() {
      lcs[i][j] = if expected[i] == actual[j] {
        lcs[i + 1][j + 1] + 1
      } else {
        lcs[i + 1][j].max(lcs[i][j + 1])
      };
    }
  }

  let mut out = String::new();
  let (mut i, mut j) = (0, 0);
  while i < n && j < m {
    if expected[i] == actual[j] {
      writeln!(out, "  {}", expected[i]).unwrap();
      i += 1;
      j += 1;
    } else if lcs[i + 1][j] >= lcs[i][j + 1] {
      writeln!(out, "- {}", expected[i]).unwrap();
      i += 1;
    } else {
      writeln!(out, "+ {}", actual[j]).unwrap();
      j += 1;
    }
  }
  for line in &expected[i..] {
    writeln!(out, "- {line}").unwrap();
  }
  for line in &actual[j..] {
    writeln!(out, "+ {line}").unwrap();
  }
  out
}
