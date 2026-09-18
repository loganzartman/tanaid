package require Tk

set gridW 32
set gridH 32
set gridRects [dict create]
set pos [list 15 15]
set ix 0
set iy 0
set dir [list 1 0]
set foodPos [list 4 4]
set len 5
set posList [list]
set frametime 80

set w 500
set h 500

canvas .c -width $w -height $h
pack .c

bind . <KeyPress-w> {
  global iy
  set iy -1
}
bind . <KeyPress-a> {
  global ix
  set ix -1
}
bind . <KeyPress-s> {
  global iy
  set iy 1
}
bind . <KeyPress-d> {
  global ix
  set ix 1
}

proc cellPx {pos} {
  global gridW gridH w h
  lassign $pos x y
  return [list \
    [expr {($x + 0.0) * $w / $gridW}] \
    [expr {($y + 0.0) * $h / $gridH}] \
    [expr {($x + 1.0) * $w / $gridW}] \
    [expr {($y + 1.0) * $h / $gridH}] \
  ]
}

# food init
lassign [cellPx $foodPos] fx1 fy1 fx2 fy2
set foodRect [.c create rectangle $fx1 $fy1 $fx2 $fy2]

proc frame {} {
  global gridW gridH gridRects pos ix iy dir foodPos len posList frametime w h foodRect

  lassign $pos x y
  lassign $dir dx dy

  # input
  if {$ix != 0} {
    set dir [list $ix 0]
    set ix 0
  } elseif {$iy != 0} {
    set dir [list 0 $iy]
    set iy 0
  }

  # move
  incr x $dx
  incr y $dy

  # wrap
  if {$x < 0} {set x [expr {$gridW - 1}]}
  if {$y < 0} {set y [expr {$gridH - 1}]}
  if {$x >= $gridW} {set x 0}
  if {$y >= $gridH} {set y 0}

  set pos [list $x $y]

  # check collision
  foreach prev $posList {
    lassign $prev px py
    if {$x == $px && $y == $py} {
      puts "game over!"
      return
    }
  }
  
  # check eatin'
  lassign $foodPos fx fy
  if {$x == $fx && $y == $fy} {
    puts "nom"
    incr len
    if {$frametime > 0} {
      incr frametime -5
    }
    # todo random
    set foodPos [list [expr {$gridW - $fx}] [expr {$gridH - $fy}]]
    lassign [cellPx $foodPos] fx1 fy1 fx2 fy2
    .c coords $foodRect $fx1 $fy1 $fx2 $fy2
  }

  # create new segment
  set newPos [list $x $y]
  lappend posList $newPos
  lassign [cellPx $newPos] rx1 ry1 rx2 ry2 
  dict set gridRects $newPos [.c create rectangle $rx1 $ry1 $rx2 $ry2]

  # remove old segments
  while {[llength $posList] > $len} {
    set posList [lassign $posList popped]
    .c delete [dict get $gridRects $popped]
  }

  after $frametime frame
}

frame
