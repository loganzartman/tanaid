proc pow {x n} {
  if {$n == 0} {
    return 1
  }
  if {$n < 0} {
    return [pow [expr {1.0 / $x}] [expr {0 - $n}]]
  }

  if {$n % 2 == 1} {
    return [expr {$x * [pow [expr {$x * $x}] [expr {($n - 1) / 2}]]}]
  } else {
    return [pow [expr {$x * $x}] [expr {$n / 2}]]
  }
}

puts [pow 2 8]
puts [pow 3 0]
puts [pow 4 -3]
