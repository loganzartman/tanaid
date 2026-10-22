package require Tk

set w 1000
set h 1000
canvas .c -width $w -height $h -background #333
pack .c

proc p {x y} {global w h; return "[expr {$x * $w}] [expr {$y * $h}]"}

proc star {c x y d1 d2 n rot closed smooth} {
  global w h
  set PI [expr {acos(-1)}]
  set TAU [expr {$PI * 2}]
  set a0 [expr {$rot + $PI / 2}]

  set i 0
  set points [list]
  while {$i < $n * 2} {
    set f [expr {double($i) / ($n * 2)}]
    set d [expr {($i % 2 == 1) * ($d2 - $d1) + $d1}]
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

  $c create line $points -width 4 -fill #fff -smooth $smooth
}

proc frame {} {
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
        star .c $x $y 0.03 0.08 $points 0 $closed $smooth
        incr i
      }
    }
  }
}

frame