set w 500
set h 500
canvas .c -width $w -height $h
pack .c

proc draw_circle {} {
  global w h
  set n 1000
  set i 0
  set t [expr {[clock clicks] / 1000000.0}]
  while {$i < $n} {
    set f [expr {double($i) / $n}]
    set x [expr {sin($f * 6.28 * sin($t * 1.2))*$w/3 + $w / 2}]
    set y [expr {cos($f * 6.28 * cos($t * 2))*$h/3 + $h / 2}]
    .c create rectangle $x $y [expr {$x + 1}] [expr {$y + 1}]
    incr i
  }
}

proc frame {} {
  .c delete all
  draw_circle
  after 1 frame
}

frame
