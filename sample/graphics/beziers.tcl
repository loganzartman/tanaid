package require Tk
set w 320
set h 240

canvas .c -width $w -height $h -background #000
pack .c
set N 16
set L 12
set speedMin 2
set speedMax 6

set i 0
set initPos [list ]
set initVel [list ]
while {$i < $L} {
  lappend initPos [expr {rand() * $w}]
  lappend initPos [expr {rand() * $h}]
  set speed [expr {rand() * ($speedMax - $speedMin) + $speedMin}]
  set dir [expr {rand() * 6.28}]
  lappend initVel [expr {cos($dir) * $speed}]
  lappend initVel [expr {sin($dir) * $speed}]
  incr i
}

set curves [list [list $initPos $initVel]]

proc rgb {r g b} {
  return [format "#%02x%02x%02x" [chan $r] [chan $g] [chan $b]]
}
proc chan {x} {
  return [expr {round(max(0, min(1, $x)) * 255.)}]
}

proc frame {} {
  global curves N L speedMin speedMax w h
  after 16 frame

  lassign [lindex $curves [expr {[llength $curves] - 1}]] pos vel
  set newCurvePos [list ]
  set newCurveVel [list ]
  foreach {x y} $pos {vx vy} $vel {
    set x [expr {$x + $vx}]
    set y [expr {$y + $vy}]
    if {$x < 0 || $x > $w} {
      set vx [expr {-$vx}]
    }
    if {$y < 0 || $y > $h} {
      set vy [expr {-$vy}]
    }
    lappend newCurvePos $x
    lappend newCurvePos $y
    lappend newCurveVel $vx
    lappend newCurveVel $vy
  }
  lappend curves [list $newCurvePos $newCurveVel]

  while {[llength $curves] > $N} {
    set curves [lassign $curves _]
  }

  set t [expr {[clock milliseconds] / 1000.}]
  set color [rgb \
    [expr {sin($t * 0.217) * 0.5 + 0.5}] \
    [expr {cos($t * 0.391) * 0.5 + 0.5}] \
    [expr {sin($t * 0.103) * 0.5 + 0.5}] \
  ]

  .c delete all
  foreach {curve} $curves {
    lassign $curve pos
    lappend pos [lindex $pos 0]
    lappend pos [lindex $pos 1]
    .c create line $pos -smooth true -fill $color
  }
}
frame
