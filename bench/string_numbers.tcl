# Numbers that exist only as strings at runtime: variables, parsed list elements, and string indexing.
# Quoting builds new strings, so these values never start out numeric.
set zeros "000"
set limit "60$zeros"
set a "1[expr {1 + 1}]"
set b "2.5"
set m "1000$zeros"

# Variables holding numeric strings, read repeatedly.
set sum 0
set i 0
while {$i < $limit} {
  set sum [expr {($sum + $a) % $m}]
  set scaled [expr {$b * $a - $b}]
  incr i
}

# List elements parsed from a string, read several times each.
set data ""
set n 0
while {$n < 200} {
  set data "$data $n"
  incr n
}
set round 0
while {$round < 100} {
  foreach x $data {
    set sum [expr {($sum + $x * $x - $x) % $m}]
  }
  incr round
}

# Short-lived strings, each read once.
set digits "0123456789"
set round 0
while {$round < 3000} {
  set j 0
  while {$j < 10} {
    set sum [expr {($sum + [string index $digits $j]) % $m}]
    incr j
  }
  incr round
}

puts "$sum $scaled"
