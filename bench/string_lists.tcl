# Lists and dicts that exist only as strings, read repeatedly from a variable.
# Each read must reuse the parsed form to avoid reparsing the whole string.
set row ""
set n 0
while {$n < 100} {
  set row "$row $n"
  incr n
}
set table ""
set n 0
while {$n < 100} {
  set table "$table key$n $n"
  incr n
}

set sum 0
set round 0
while {$round < 300} {
  set i 0
  while {$i < [llength $row]} {
    set sum [expr {$sum + [lindex $row $i] + [dict get $table key$i]}]
    incr i
  }
  incr round
}

puts $sum
