# The per-frame update from sample/fireworks.tcl without Tk: float math and in-place
# list updates (lindex, lassign, lset) over a list of 400 particles.
set state 31
proc randint {} {
  global state
  set state [expr {($state * 1103515245 + 12345) % 2147483648}]
  return $state
}
proc random {{a 1} {b "none"}} {
  if {$b == "none"} {
    set b $a
    set a 0
  }
  return [expr {[randint] / 2147483648.0 * ($b - $a) + $a}]
}

set particles [list]
set n 0
while {$n < 400} {
  lappend particles [list $n [random 500] [random 500] [random -5 5] [random -5 5]]
  incr n
}

proc frame {} {
  global particles
  set i 0
  while {$i < [llength $particles]} {
    lassign [lindex $particles $i] id x y vx vy
    set x [expr {$x + $vx}]
    set y [expr {$y + $vy}]
    set vy [expr {$vy + 0.25}]
    set vx [expr {$vx * 0.99}]
    set vy [expr {$vy * 0.99}]
    lset particles $i 1 $x
    lset particles $i 2 $y
    lset particles $i 3 $vx
    lset particles $i 4 $vy
    incr i
  }
}

set f 0
while {$f < 100} {
  frame
  incr f
}

puts [lindex $particles 0]
