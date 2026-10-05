package require Tk
set w 256
set h 256
canvas .c -width $w -height $h
pack .c
set r [.c create rectangle 0 0 0 0]
set p [list 128 128]
set v [list 3 -6]
set rw 8
set rh 8

proc frame {} {
  global r p v w h rw rh
  after 15 frame
  
  lassign $p x y
  lassign $v vx vy
  set x [expr {$x + $vx}]
  set y [expr {$y + $vy}]
  set vy [expr {$vy + 0.5}]

  if {$x < 0} {set x 0; set vx [expr {-$vx}]}
  if {$x > $w} {set x $w; set vx [expr {-$vx}]}
  if {$y > $h} {set y $h; set vy [expr {-$vy}]}
  if {$vy > 12} {set vy 12}

  set v [list $vx $vy]
  set p [list $x $y]
  
  .c coords $r [expr {$x - $rw / 2}] [expr {$y - $rh / 2}] [expr {$x + $rw / 2}] [expr {$y + $rh / 2}]
}
frame

vwait forever
