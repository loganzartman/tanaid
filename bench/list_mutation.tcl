# Growing and updating lists and dicts held in variables. Each update changes one element.
set items [list]
set i 0
while {$i < 3000} {
  lappend items $i
  incr i
}

set round 0
while {$round < 2} {
  set i 0
  while {$i < 3000} {
    lset items $i [expr {[lindex $items $i] + 1}]
    incr i
  }
  incr round
}

set counts [dict create]
set i 0
while {$i < 1000} {
  dict set counts key$i $i
  incr i
}

puts "[llength $items] [lindex $items 2999] [dict get $counts key999]"
