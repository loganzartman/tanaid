package require Tk

proc bez {a c1 c2 b t} {
  set l11 [lerp $a $c1 $t]
  set l12 [lerp $c1 $c2 $t]
  set l13 [lerp $c2 $b $t]
  set l21 [lerp $l11 $l12 $t]
  set l22 [lerp $l12 $l13 $t]
  return [lerp $l21 $l22 $t]
}

proc lerp {a b t} {
  lassign $a x1 y1
  lassign $b x2 y2
  set x [expr {$x1 + ($x2 - $x1) * $t}]
  set y [expr {$y1 + ($y2 - $y1) * $t}]
  return [p $x $y]
}

proc p {a b} {return [list $a $b]}

set w 500
set h 500
canvas .c -width $w -height $h
pack .c

set i 0
while {$i <= 100} {
  .c create rectangle 0 0 0 0
  incr i
}

proc frame {} {
  global w h
  after 16 frame

  set t [expr {[clock monotonic] / 1000000.}]
  set i 0
  while {$i <= 100} {
    set f [expr {$i / 100.0 * 0.6 + $t}]
    # why is my modulo broken lol
    while {$f > 1.0} {set f [expr {$f - 1.0}]}
    lassign [bez [p 0 [expr {$h * 0.5}]] [p [expr {$w * 0.33}] $h] [p [expr {$w * 0.66}] 0] [p $w [expr {$h * 0.5}]] $f] x y
    .c coords $i $x $y [expr {$x + 2}] [expr {$y + 2}]
    incr i 1
  }
}
frame
