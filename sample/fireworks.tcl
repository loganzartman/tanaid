package require Tk

set w 500
set h 500
canvas .c -width $w -height $h
pack .c

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

set part [list]
set rect [list]

proc frame {} {
  after 16 frame
  global part rect
  set i 0
  while {$i < [llength $part]} {
    lassign [lindex $part $i] x y vx vy
    set r [lindex $rect $i]

    set x [expr {$x + $vx}]
    set y [expr {$y + $vy}]
    set vy [expr {$vy + 0.25}]
    set vx [expr {$vx * 0.99}]
    set vy [expr {$vy * 0.99}]
    lset part $i 0 $x
    lset part $i 1 $y
    lset part $i 2 $vx
    lset part $i 3 $vy

    .c coords $r $x $y [expr {$x + 8}] [expr {$y + 8}]
    incr i
  }
}

proc launch {} {
  global part rect w h
  after [expr {[randint] % 1500 + 500}] launch

  set x [random $w]
  set y [random $h]

  set i 0
  while {$i < 20} {
    lappend part [list $x $y [expr {[random -5 5]}] [expr {[random -5 5]}]]
    lappend rect [.c create rectangle 0 0 0 0]
    incr i
  }
}

launch
frame

