package require Tk
set w 256
set h 256
canvas .c -width $w -height $h
pack .c
set rect [.c create rectangle 0 0 0 0]
set pos [list 128 128]
set vel [list 5 -6]
set stretch 1
set rw 8
set rh 8

proc frame {} {
  global rect pos vel stretch w h rw rh
  after 15 frame
  
  lassign $pos x y
  lassign $vel vx vy
  set x [expr {$x + $vx}]
  set y [expr {$y + $vy}]
  set vy [expr {$vy + 0.5}]
  set stretch [expr {$stretch * 0.9 + 0.1}]

  if {$x < 0} {set x 0; set vx [expr {-$vx}]; set stretch 3}
  if {$x > $w} {set x $w; set vx [expr {-$vx}]; set stretch 3}
  if {$y > $h} {set y $h; set vy -15; set stretch 3}

  set vel [list $vx $vy]
  set pos [list $x $y]

  .c coords $rect \
    [expr {$x - $rw * $stretch / 2}] \
    [expr {$y - $rh * $stretch / 2}] \
    [expr {$x + $rw * $stretch / 2}] \
    [expr {$y + $rh * $stretch / 2}]
}
frame

vwait forever
