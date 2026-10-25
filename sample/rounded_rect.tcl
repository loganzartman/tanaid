package require Tk

# move mouse left and right to change roundness

proc roundrect {canvas x1 y1 x2 y2 radius fill width} {
  set w [expr {$x2 - $x1}]
  set h [expr {$y2 - $y1}]
  set rw [expr {min($w / 2, max(0, $radius))}]
  set rh [expr {min($h / 2, max(0, $radius))}]
  
  set coords [list \
    $x1 $y1 \
    [expr {$x1 + $rw}] $y1 \
    [expr {$x2 - $rw}] $y1 \
    $x2 $y1 \
    $x2 [expr {$y1 + $rh}] \
    $x2 [expr {$y2 - $rh}] \
    $x2 $y2 \
    [expr {$x2 - $rw}] $y2 \
    [expr {$x1 + $rw}] $y2 \
    $x1 $y2 \
    $x1 [expr {$y2 - $rh}] \
    $x1 [expr {$y1 + $rh}] \
    $x1 $y1 \
  ]

  set result [$canvas create line $coords -smooth true -fill $fill -width $width]

  # debug
  foreach {x y} $coords {
    $canvas create rectangle [expr {$x - 2}] [expr {$y - 2}] [expr {$x + 2}] [expr {$y + 2}] -fill #414 -outline {}
  }
  
  return $result
}

canvas .c -width 250 -height 250 -background #cbc
pack .c

proc draw {r} {
  .c delete all
  roundrect .c 50 50 200 200 $r #848 2
}
draw 32

bind . <Motion> {
  draw [expr {%x / 250.0 * 80}]
}
