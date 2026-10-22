# Character counting: string indexing and dictionary reads and writes.
set text "the quick brown fox jumps over the lazy dog"
set k 0
while {$k < 3000} {
  set freq [dict create]
  set i 0
  while {$i < [string length $text]} {
    set char [string index $text $i]

    if {[dict exists $freq $char]} {
      dict set freq $char [expr {[dict get $freq $char] + 1}]
    } else {
      dict set freq $char 1
    }

    incr i
  }
  incr k
}

# print fixed keys: dict iteration order isn't the same in tanaid and tclsh
puts "o=[dict get $freq o] e=[dict get $freq e] space=[dict get $freq { }]"
