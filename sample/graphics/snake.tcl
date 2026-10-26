package require Tk

set gridW 25
set gridH 25
set gridRects [dict create]
set pos [list 15 15]
set ix 0
set iy 0
set dir [list 1 0]
set foodPos [list 4 4]
set len 5
set score 0
set posList [list]
set speed 0.0

set w 500
set h 500

canvas .c -width $w -height $h -background #111
pack .c

bind . <KeyPress> {
  global ix iy
  if {%K == "w" || %K == "Up"} {set iy -1}
  if {%K == "a" || %K == "Left"} {set ix -1}
  if {%K == "s" || %K == "Down"} {set iy 1}
  if {%K == "d" || %K == "Right"} {set ix 1}
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
set foodRect [.c create rectangle [cellPx $foodPos] -fill #944231 -outline {}]

proc frame {} {
  global gridW gridH gridRects pos ix iy dir foodPos len score posList speed w h foodRect

  lassign $pos x y
  lassign $dir dx dy

  # input
  if {$ix != 0} {
    # prevent 180 turn
    if {[lindex $dir 0] == 0} {
      set dir [list $ix 0]
    }
    set ix 0
  } elseif {$iy != 0} {
    if {[lindex $dir 1] == 0} {
      set dir [list 0 $iy]
    }
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
    incr len
    incr score
    set speed [expr {$speed + (1 - $speed) * 0.04}]
    set foodPos [list [expr {int(rand() * $gridW)}] [expr {int(rand() * $gridH)}]]
    .c coords $foodRect [cellPx $foodPos]
    puts "score: $score | speed: [expr {int($speed * 100)}]"
  }

  # create new segment
  set newPos [list $x $y]
  lappend posList $newPos
  dict set gridRects $newPos [.c create rectangle [cellPx $newPos] -fill #31893D -outline {}]

  # remove old segments
  while {[llength $posList] > $len} {
    set posList [lassign $posList popped]
    .c delete [dict get $gridRects $popped]
  }

  after [expr {int(100 - $speed * 65)}] frame
}

frame
vwait forever
