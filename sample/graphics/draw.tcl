package require Tk

canvas .c -width 500 -height 500
pack .c

set down false
set color #121212
set width 6

set cursor -1
set lx 0
set ly 0
set cx 0 
set cy 0

proc drawCursor {} {
  global cursor cx cy width color
  .c delete $cursor
  set cursor [.c create rectangle \
    [expr {$cx - $width / 2}] \
    [expr {$cy - $width / 2}] \
    [expr {$cx + $width / 2}] \
    [expr {$cy + $width / 2}] \
    -outline $color]
}

bind . <Key-1> {global color; set color #121212; drawCursor}
bind . <Key-2> {global color; set color #6d1f1b; drawCursor}
bind . <Key-3> {global color; set color #1b4b01; drawCursor}
bind . <Key-4> {global color; set color #2a3778; drawCursor}
bind . <Key-comma> {global width; set width [expr {max(1, $width - 1)}]; drawCursor}
bind . <Key-period> {global width; set width [expr {min(16, $width + 1)}]; drawCursor}

bind . <ButtonPress> {
  global down color width line lx ly
  set lx %x
  set ly %y
  set line [.c create line {%x %y %x %y} -smooth true -width $width -fill $color]
  set down true
}

bind . <ButtonRelease> {
  global down
  set down false
}

bind . <Motion> {
  global line lx ly width
  
  if {$down} {
    if {[expr {hypot($lx - %x, $ly - %y) >= 5}]} {
      set lx %x
      set ly %y
      
      set coords [.c coords $line]
      lappend coords %x
      lappend coords %y
      .c coords $line $coords
    }
  }

  set cx %x
  set cy %y
  drawCursor
}
