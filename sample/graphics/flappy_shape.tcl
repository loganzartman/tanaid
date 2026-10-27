package require Tk

proc = {var ex} {uplevel 1 "set $var \[expr {$ex}\]"}

set w 300
set h 400
set G 800
set birdY 0
set birdVy 0
set jumpSpeed -300
set pipeW 38
set difficulty 0
set pipes [list]

canvas .c -width $w -height $h -background #71c5cf
pack .c

set canJump true
bind . <ButtonPress> {
  global birdVy jumpSpeed
  set birdVy $jumpSpeed
}
bind . <KeyPress> {
  global canJump birdVy jumpSpeed
  if {$canJump && (%K == "space" || %K == "Up")} {
    set birdVy $jumpSpeed
    set canJump false
  }
}
bind . <KeyRelease> {
  global canJump
  if {%K == "space" || %K == "Up"} {
    set canJump true
  }
}

proc init {} {
  global w h birdY birdVy pipes pipeW tstart
  = birdY {$h / 2}
  = birdVy {0}
  set i 0
  set n 2

  set tstart [clock milliseconds]
  set pipes [list]
  while {$i < $n} {
    = x {$w * 1.2 + $i * (($w + $pipeW) / $n)}
    lappend pipes [list $x [expr {$h / 2}] [expr {$h * 0.6}]]
    incr i
  }
}

proc drawPipe {c x gapY gapH} {
  global h pipeW
  = x1 {$x - $pipeW / 2 + 4}
  = x2 {$x + $pipeW / 2 - 4}
  = yTop {$gapY - $gapH / 2}
  = yBot {$gapY + $gapH / 2}
  $c create rectangle $x1 -4 $x2 $yTop -fill #74bf2e
  $c create rectangle $x1 $yBot $x2 [expr {$h+4}] -fill #74bf2e
  = capX1 {$x1 - 4}
  = capX2 {$x2 + 4}
  = capTopY1 {$yTop - 16}
  = capBotY2 {$yBot + 16}
  $c create rectangle $capX1 $capTopY1 $capX2 $yTop -fill #74bf2e
  $c create rectangle $capX1 $yBot $capX2 $capBotY2 -fill #74bf2e
}

set tstart [clock milliseconds]
set t0 [clock milliseconds]

proc step {} {
  global w h birdY birdVy difficulty G pipes pipeW t0 tstart
  after 16 step

  set t1 [clock milliseconds]
  = telapsed {($t1 - $tstart) / 1000.}
  = dt {($t1 - $t0) / 1000.}
  set t0 $t1

  = difficulty {pow(min(1, $telapsed / (60 * 2)), 2.)}
  = speed {70 + $difficulty * 100}
  = gap {$h * (0.6 - $difficulty * 0.43)}

  = birdVy {$birdVy + $G * $dt}
  = birdY {$birdY + $birdVy * $dt}

  = birdX1 {$pipeW * 1.5}
  = birdX2 {$birdX1 + $h * 0.06}
  = birdY1 {$birdY - $h * 0.06 * 0.5}
  = birdY2 {$birdY + $h * 0.06 * 0.5}

  .c delete all 

  set newpipes [list]
  foreach pipe $pipes {
    lassign $pipe x gapY gapH
    drawPipe .c $x $gapY $gapH

    = x {$x - $speed * $dt}

    if {$x + $pipeW / 2 > 0} {
      lappend newpipes [list $x $gapY $gapH]
    } else {
      lappend newpipes [list [expr {$w - $x}] [expr {int(rand() * $h)}] $gap]
    }
  }
  set pipes $newpipes

  .c create rectangle $birdX1 $birdY1 $birdX2 $birdY2 -fill #d5c42b

  if {$birdY1 > $h} {
    puts "Game over! Score: [expr {int($telapsed)}]"
    init
  }
}

init
step
