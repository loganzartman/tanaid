# A tight while loop of integer arithmetic at global scope: expr, if, incr, set.
set i 0
set sum 0
while {$i < 200000} {
  set sum [expr {($sum + $i * 3) % 1000003}]
  if {$sum % 2 == 0} {
    incr sum
  }
  incr i
}

puts $sum
