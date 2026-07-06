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
