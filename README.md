# tanaid

i am learning rust and tcl by writing a tcl interpreter. i chose tcl because it is "very simple" and actually used for some things.

implements a (probably broken) subset of tcl:

- syntax
  - bare `words`
  - `$variable` substitution
  - `{braced words}`
  - `${braced variable}` substitution
  - `[command args]` substituion
  - `"quoted strings"`
  - `"$variable and [command]"` substitution in quoted strings
- builtin commands
  - `break`
  - `dict`
    - `dict create ?key value ...?`
    - `dict get dictValue key`
    - `dict has dictValue key`
    - `dict set dictVariable key value`
  - `expr arg ?arg arg ...?`
  - `global ?varname ...?`
  - `if expr1 ?then? body1 elseif expr2 ?then? body2 elseif ... ?else? ?bodyN?`
  - `info exists varName`
  - `proc name args body`
  - `puts string`
  - `return ?result?`
  - `set varName ?value?`
  - `while test body`

this is enough to write simple scripts like:

```tcl
proc fib {x} {
  if {$x <= 0} {
    return 0
  }
  if {$x == 1} {
    return 1
  }
  return [expr {[fib [expr {$x - 1}]] + [fib [expr {$x - 2}]]}]
}

set i 1
while {$i < 30} {
  puts [fib $i]
  set i [expr {$i + 1}]
}
```

## Usage

```sh
cargo build --release

# repl
./target/release/tanaid

# run a script
./target/release/tanaid path/to/file.tcl
```

## Setup

Install Rust with `rustup`:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Run any Cargo command from the repo root to install/use the toolchain:

```sh
cargo build
```

## Development

Run the compiler checks with:

```sh
cargo check --all-targets
```
