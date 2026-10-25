package require Tk

# params
set w 500
set h 500
set fps 60
set G 2000
set nnodes 32
set stiffness 01
set drag 5.0
set substeps 3
set length [expr {hypot($w/2, $h/2)}]

# state
set nodedist [expr {$length / $nnodes}]
set nodepos [list ]
set nodevel [list ]

canvas .c -width $w -height $h -background #000
pack .c

set mx 0
set my 0

bind . <Motion> {
  global mx my
  set mx %x
  set my %y
}

set pinx [expr {$w / 2}]
set piny [expr {$h / 2}]

bind . <ButtonPress> {
  global pinx piny
  set pinx %x
  set piny %y
}

# init
set i 0
while {$i < $nnodes} {
  lappend nodepos [list [expr {$w / 2 + $i * $nodedist}] [expr {$h / 2}]]
  lappend nodevel [list 0 0]
  incr i
}

proc time {} {return [expr [clock milliseconds] / 1000.0]}

set t0 [time]
proc frame {} {
  global w h fps G mx my pinx piny pinned nnodes nodedist nodepos nodevel stiffness drag substeps t0

  after [expr {int(1000/$fps)}] frame
  
  set t1 [time]
  set dt [expr {($t1 - $t0)}]
  set t0 $t1

  .c delete all

  set dt [expr {$dt / $substeps}]
  set step 0
  while {$step < $substeps} {
    set i 0
    while {$i < $nnodes} {
      set b [lindex $nodepos $i]
      set vb [lindex $nodevel $i]
  
      lassign $b bx by
      lassign $vb vbx vby
  
      # time integration  
      set vby [expr {$vby + $G * $dt}]
      set bx [expr {$bx + $vbx * $dt}]
      set by [expr {$by + $vby * $dt}]
  
      # drag
      set vbx [expr {$vbx - $vbx * $drag * $dt}]
      set vby [expr {$vby - $vby * $drag * $dt}]
  
      lset nodepos $i [list $bx $by]
      lset nodevel $i [list $vbx $vby]
      incr i
    }
  
    set i 1
    while {$i < $nnodes} {
      set a [lindex $nodepos [expr {$i - 1}]]
      set b [lindex $nodepos $i]
      set va [lindex $nodevel [expr {$i - 1}]]
      set vb [lindex $nodevel $i]
  
      lassign $a ax ay
      lassign $b bx by
      lassign $va vax vay
      lassign $vb vbx vby
  
      # constrain length
      set dx [expr {$bx - $ax}]
      set dy [expr {$by - $ay}]
      set len [expr {hypot($dx, $dy)}]
  
      # correction offset
      set ox [expr {($ax + ($dx / $len) * $nodedist) - $bx}]
      set oy [expr {($ay + ($dy / $len) * $nodedist) - $by}]

      set wa 0.5
      set wb 0.5
      if {$i == 1} {
        set wa 0
        set wb 1
      } elseif {$i == $nnodes - 1} {
        set wa 1
        set wb 0
      }

      # compute corrected positions
      set axc [expr {$ax - $ox * $wa * $stiffness}]
      set ayc [expr {$ay - $oy * $wa * $stiffness}]
      set bxc [expr {$bx + $ox * $wb * $stiffness}]
      set byc [expr {$by + $oy * $wb * $stiffness}]
  
      # apply correction force based on movement
      set vax [expr {$vax + ($axc - $ax) / $dt}]
      set vay [expr {$vay + ($ayc - $ay) / $dt}]
      set vbx [expr {$vbx + ($bxc - $bx) / $dt}]
      set vby [expr {$vby + ($byc - $by) / $dt}]
  
      lset nodepos [expr {$i - 1}] [list $axc $ayc]
      lset nodevel [expr {$i - 1}] [list $vax $vay]
      lset nodepos $i [list $bxc $byc]
      lset nodevel $i [list $vbx $vby]
      incr i
    }
    incr step
  }

  # pin nodes
  lset nodepos 0 [list $mx $my]
  lset nodevel 0 [list 0 0]
  lset nodepos [expr {$nnodes - 1}] [list $pinx $piny]
  lset nodevel [expr {$nnodes - 1}] [list 0 0]

  set i 0
  set points [lindex $nodepos 0]
  while {$i < $nnodes} {
    set a [lindex $nodepos [expr {$i - 1}]]
    set b [lindex $nodepos $i]

    lassign $a ax ay
    lassign $b bx by

    lappend points $bx
    lappend points $by

    if {$i == 0} {
      .c create rectangle [expr {$bx - 2}] [expr {$by - 28}] [expr {$bx + 2}] [expr {$by - 16}] -fill #bdad8f -width 0
    }
    if {$i == 0 || $i == $nnodes - 1} {
      .c create rectangle [expr {$bx - 4}] [expr {$by - 16}] [expr {$bx + 4}] [expr {$by + 4}] -fill #e9db04 -width 0
    }
    incr i
  }
  .c create line $points -fill #e9db04 -width 5 -smooth true

}

after 10 frame
