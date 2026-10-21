# Counting characters, from sample/count_letter_freq.tcl: string index, dict get/set.
proc countLetterFreq {s} {
  set i 0
  set len [string length $s]
  set freq [dict create]

  while {$i < $len} {
    set char [string index $s $i]

    if {[dict exists $freq $char]} {
      dict set freq $char [expr {[dict get $freq $char] + 1}]
    } else {
      dict set freq $char 1
    }

    set i [expr {$i + 1}]
  }

  return $freq
}

set text "the quick brown fox jumps over the lazy dog"
set k 0
while {$k < 3000} {
  countLetterFreq $text
  incr k
}

# print fixed keys: dict iteration order isn't the same in tanaid and tclsh
set freq [countLetterFreq $text]
puts "o=[dict get $freq o] e=[dict get $freq e] space=[dict get $freq { }]"
