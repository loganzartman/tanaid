package require Tk

proc = {var ex} {uplevel 1 "set $var \[expr {$ex}\]"}

set w 300
set h 400
set fps 60
set G 1000
set birdY 0
set birdVy 0
set canJump true
set jumpSpeed -400
set pipeW 38
set paused true
set pipes [list]
set tstart 0
set t0 0

canvas .c -width $w -height $h -background #71c5cf
pack .c

proc unpause {} {
  global paused t0 tstart
  if {!$paused} {
    return
  }
  set paused false
  set tstart [clock milliseconds]
  set t0 [clock milliseconds]
}

bind . <ButtonPress> {
  global birdVy jumpSpeed
  unpause
  set birdVy $jumpSpeed
}

bind . <KeyPress> {
  global canJump birdVy jumpSpeed
  unpause
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
  global w h birdY birdVy pipes pipeW paused score tstart
  set paused true
  set score 0
  set i 0
  set n 2
  set tstart [clock milliseconds]
  set pipes [list]
  = birdY {$h / 2}
  = birdVy {0}

  # create initial pipes
  while {$i < $n} {
    = x {$w * 1.2 + $i * (($w + $pipeW) / $n)}
    lappend pipes [list $x [expr {$h / 2}] [expr {$h * 0.6}]]
    incr i
  }
}

proc gameOver {} {
  global score
  puts "Game over! Score: $score"
  init
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

proc drawPaused {c} {
  global w h
  $c create rectangle [expr {$w * 0.5 - 16}] [expr {$h * 0.5 - 24}] [expr {$w * 0.5 - 4}] [expr {$h * 0.5 + 24}] -fill #fff -width 3
  $c create rectangle [expr {$w * 0.5 + 4}] [expr {$h * 0.5 - 24}] [expr {$w * 0.5 + 16}] [expr {$h * 0.5 + 24}] -fill #fff -width 3
}

proc getBirdPos {} {
  global pipeW birdX birdY h
  = birdX1 {$pipeW * 1.5}
  = birdX2 {$birdX1 + $h * 0.05}
  = birdY1 {$birdY - $h * 0.05 * 0.5}
  = birdY2 {$birdY + $h * 0.05 * 0.5}
  return [list $birdX1 $birdY1 $birdX2 $birdY2]
}

proc stepUpdate {} {
  global birdY birdVy pipeW pipes G w h score tstart t0
  set t1 [clock milliseconds]
  = telapsed {($t1 - $tstart) / 1000.}
  = dt {($t1 - $t0) / 1000.}
  set t0 $t1

  # compute difficulty params
  = difficulty {pow(min(1, $telapsed / (60 * 2)), 2.)}
  = speed {70 + $difficulty * 100}
  = gap {$h * (0.6 - $difficulty * 0.43)}

  # bird time integration
  = birdVy {$birdVy + $G * $dt}
  = birdY {$birdY + $birdVy * $dt}

  lassign [getBirdPos] birdX1 birdY1 birdX2 birdY2

  # update pipes
  set newpipes [list]
  foreach pipe $pipes {
    lassign $pipe x gapY gapH
    = x {$x - $speed * $dt}

    # check bird-pipe collision
    if {\
      ($birdY2 >= $gapY + $gapH / 2 || $birdY1 <= $gapY - $gapH / 2) && \
      ($birdX2 >= $x - $pipeW / 2 && $birdX1 <= $x + $pipeW / 2) \
    } {
      gameOver
      return
    }

    # check pipe on-screen
    if {$x + $pipeW / 2 > 0} {
      lappend newpipes [list $x $gapY $gapH]
    } else {
      # pipe moved off left side
      incr score
      lappend newpipes [list [expr {$w - $x}] [expr {int(rand() * $h * 0.7 + 0.15)}] $gap]
    }
  }
  set pipes $newpipes

  # check bird-ground collision
  if {$birdY1 > $h} {
    gameOver
    return
  }
}

proc stepDraw {} {
  global pipes paused
  .c delete all

  foreach pipe $pipes {
    lassign $pipe x gapY gapH
    drawPipe .c $x $gapY $gapH
  }

  lassign [getBirdPos] birdX1 birdY1 birdX2 birdY2
  .c create rectangle $birdX1 $birdY1 $birdX2 $birdY2 -fill #d5c42b

  if {$paused} {
    drawPaused .c
  }
}

proc step {} {
  global fps paused
  after [expr {1000 / $fps}] step

  if {!$paused} {
    stepUpdate
  }
  stepDraw
}

init
step
