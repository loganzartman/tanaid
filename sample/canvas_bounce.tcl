# a rectangle bouncing back and forth across a canvas

set width 400
set height 300

canvas .c -width $width -height $height -background #1d1f21
pack .c

set box [.c create rectangle 20 110 140 190 -fill #e2725b]
set dx 4

proc step {} {
  global box dx width

  set coords [.c coords $box]
  set x1 [lindex $coords 0]
  set x2 [lindex $coords 2]

  if {$x2 + $dx > $width || $x1 + $dx < 0} {
    set dx [expr {0 - $dx}]
  }

  .c move $box $dx 0
  after 16 step
}

step
