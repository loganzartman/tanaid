# Float math and in-place nested list updates over 400 particles.
set particles [list]
set n 0
while {$n < 400} {
  lappend particles [list $n $n 2.5 -2.5]
  incr n
}

proc frame {} {
  global particles
  set i 0
  while {$i < [llength $particles]} {
    lassign [lindex $particles $i] x y vx vy
    set x [expr {$x + $vx}]
    set y [expr {$y + $vy}]
    set vy [expr {$vy + 0.25}]
    set vx [expr {$vx * 0.99}]
    set vy [expr {$vy * 0.99}]
    lset particles $i 0 $x
    lset particles $i 1 $y
    lset particles $i 2 $vx
    lset particles $i 3 $vy
    incr i
  }
}

set f 0
while {$f < 100} {
  frame
  incr f
}

puts [lindex $particles 0]
