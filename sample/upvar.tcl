proc decr {varName decrement} {
  upvar 1 $varName var
  incr var [expr {0 - $decrement}]
}

set x 10
decr x 1
decr x 2
puts $x
