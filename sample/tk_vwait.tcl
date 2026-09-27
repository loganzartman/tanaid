package require Tk
canvas .c -width 200 -height 200
pack .c
proc tick {} { puts tick; after 400 tick }
tick
vwait forever
