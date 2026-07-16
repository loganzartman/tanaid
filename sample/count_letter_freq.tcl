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

puts [countLetterFreq hello]
puts [countLetterFreq racecar]
puts [countLetterFreq "a gentleman"]
puts [countLetterFreq "elegant man"]
