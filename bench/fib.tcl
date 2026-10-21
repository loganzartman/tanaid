# Recursive proc calls: argument binding, frames, return, small exprs.
proc fib {x} {
  if {$x < 2} {
    return $x
  }
  return [expr {[fib [expr {$x - 1}]] + [fib [expr {$x - 2}]]}]
}

puts [fib 23]
