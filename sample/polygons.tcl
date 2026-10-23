package require Tk

set w 1000
set h 1000
canvas .c -width $w -height $h -background #333
pack .c

set mx 0
set my 0
bind . <Motion> {
  global mx my w h
  set mx [expr {double(%x) / $w}]
  set my [expr {double(%y) / $h}]
}

proc p {x y} {global w h; return "[expr {$x * $w}] [expr {$y * $h}]"}

proc star {c x y r1 r2 n rot closed smooth fill} {
  global w h
  set PI [expr {acos(-1)}]
  set TAU [expr {$PI * 2}]
  set a0 [expr {$rot + $PI / 2}]

  set i 0
  set points [list]
  while {$i < $n * 2} {
    set f [expr {double($i) / ($n * 2)}]
    set d [expr {($i % 2 == 1) * ($r2 - $r1) + $r1}]
    set px [expr {($x + cos($f * $TAU + $a0) * $d) * $w}]
    set py [expr {($y + sin($f * $TAU + $a0) * $d) * $h}]
    lappend points $px
    lappend points $py
    incr i
  }
  
  if {$closed} {
    lappend points [lindex $points 0]
    lappend points [lindex $points 1]
  }

  $c create line $points -width 4 -fill $fill -smooth $smooth
}

proc frame {} {
  global mx my
  after 16 frame
  set t [expr {[clock milliseconds] / 1000.0}]

  .c delete all

  set nx 3
  set ny 4
  set i 0

  foreach points {3 5} {
    foreach closed {true false} {
      foreach smooth {false true raw} {
        set x [expr {(1.0 / ($nx + 1)) * ($i % $nx + 1)}]
        set y [expr {(1.0 / ($ny + 1)) * ($i / $nx + 1)}]
        set mouseFactor [expr {1 - min(1, hypot($x - $mx, $y - $my) / 0.4)}]
        set d1 0.03
        set d2 [expr {0.07 + 0.05 * $mouseFactor}]
        set angle [expr {sin($t * 3) * 0.1}]
        set fill "#fac"
        star .c $x $y $d1 $d2 $points $angle $closed $smooth $fill
        incr i
      }
    }
  }
}

frame