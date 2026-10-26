proc do {body while condition} {
  if {$while != "while"} {
    error "required word missing"
  }
  set conditionCmd [list expr $condition]
  while {1} {
    uplevel 1 $body
    if {[uplevel 1 $conditionCmd]} then {
    } else {
      break
    }
  }
}

set i 0
do {
  puts $i
  incr i
} while {$i < 5}
