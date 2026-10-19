package require Tk

canvas .c -width 500 -height 500
pack .c

set down false

bind . <ButtonPress> {
  global down
  set down true
}

bind . <ButtonRelease> {
  global down
  set down false
}

bind . <Motion> {
  global down
  if {$down} {
    .c create rectangle [expr {%x - 2}] [expr {%y - 2}] [expr {%x + 2}] [expr {%y + 2}]
  }
}
