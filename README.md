# tanaid

[_tuh-NAY-id_](https://en.wikipedia.org/wiki/Tanaidacea)

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
  - [`break`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/break.html)
  - [`continue`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/continue.html)
  - [`dict create ?key value ...?`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/dict.html#M6:~:text=dict%20create%20%3Fkey%20value%20...%3F,-Return%20a)
  - [`dict get dictValue ?key ...?`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/dict.html#M13:~:text=dict%20get%20dictionaryValue%20%3Fkey%20...%3F,-Given%20a)
  - [`dict exists dictValue key ?key ...?`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/dict.html#M7:~:text=dict%20exists%20dictionaryValue%20key%20%3Fkey%20...%3F,-This%20returns)
  - [`dict set dictVariable key ?key ...? value`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/dict.html#M24:~:text=dict%20set%20dictionaryVariable%20key%20%3Fkey%20...%3F%20value,-This%20operation)
  - [`expr arg ?arg arg ...?`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/expr.html)
  - [`foreach varlist1 list1 ?varlist2 list2 ...? body`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/foreach.html)
  - [`global ?varname ...?`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/global.html)
  - [`if expr1 ?then? body1 elseif expr2 ?then? body2 elseif ... ?else? ?bodyN?`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/if.html)
  - [`incr varName ?increment?`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/incr.html)
  - [`info exists varName`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/info.html#M27:~:text=info%20exists%20varName,-Returns%201)
  - [`lappend listVar ?value value value ...?`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/lappend.html)
  - [`lindex listVal index`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/lindex.html)
  - [`list ?arg arg ...?`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/list.html)
  - [`llength listVal`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/llength.html)
  - [`lreverse listVal`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/lreverse.html)
  - [`proc name args body`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/proc.html)
  - [`puts string`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/puts.html)
  - [`return ?result?`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/return.html)
  - [`set varName ?value?`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/set.html)
  - [`string index string charIndex`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/string.html#M9:~:text=string%20index%20string%20charIndex,-Returns%20the)
  - [`string length string`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/string.html#M35:~:text=string%20length%20string,-Returns%20a)
  - [`unknown cmdName ?arg arg ...?`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/unknown.html)
  - [`while test body`](https://www.tcl-lang.org/man/tcl9.0/TclCmd/while.html)

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
