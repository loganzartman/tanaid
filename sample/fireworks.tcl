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

set particles [list]

proc frame {} {
  after 16 frame
  global particles
  
  set i 0
  while {$i < [llength $particles]} {
    lassign [lindex $particles $i] rect x y vx vy

    set x [expr {$x + $vx}]
    set y [expr {$y + $vy}]
    set vy [expr {$vy + 0.25}]
    set vx [expr {$vx * 0.99}]
    set vy [expr {$vy * 0.99}]
    lset particles $i 1 $x
    lset particles $i 2 $y
    lset particles $i 3 $vx
    lset particles $i 4 $vy

    .c coords $rect $x $y [expr {$x + 8}] [expr {$y + 8}]
    incr i
  }
}

proc launch {} {
  global particles w h
  after [expr {[randint] % 1500 + 500}] launch

  set x [random $w]
  set y [random $h]

  set i 0
  while {$i < 20} {
    set vx [expr {[random -5 5]}]
    set vy [expr {[random -5 5]}]
    lappend particles [list [.c create rectangle 0 0 0 0] $x $y $vx $vy]
    incr i
  }
}

launch
frame
