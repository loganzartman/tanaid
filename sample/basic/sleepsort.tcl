proc sleepsort {l} {
  global result remaining
  set result [list]
  set remaining [llength $l]

  proc append {x} {
    global result remaining
    lappend result $x
    incr remaining -1
    puts $x
  }

  foreach x $l {
    after $x "append $x"
  }

  while {$remaining > 0} {
    vwait remaining
  }
  return $result
}

puts "Sorted: [sleepsort [list 100 500 300 200 800]]"
