proc shortestPath {graph from to} {
  set queue [list $from]
  set head 0
  set seen [dict create $from 1]
  set previous [dict create]

  while {$head < [llength $queue]} {
    set node [lindex $queue $head]
    incr head

    if {$node == $to} {
      return [rebuildPath $previous $node $from]
    }

    if [dict exists $graph $node] {
      set edges [dict get $graph $node]

      foreach toNode $edges {
        if {[dict exists $seen $toNode]} {
          continue
        }

        dict set seen $toNode 1
        dict set previous $toNode $node
        lappend queue $toNode
      }
    }
  }

  return {}
}

proc rebuildPath {previous node from} {
  set path {}
  while {1} {
    lappend path $node
    if {$node == $from} {
      break
    }
    set node [dict get $previous $node]
  }
  return [lreverse $path]
}

set graph {a {b c} b {c d} d {e} e {f} c {f}}
puts [shortestPath $graph a f]
puts [shortestPath $graph a e]
puts [shortestPath $graph a z]
puts [shortestPath $graph a a]

set cyclicGraph {a {b} b {c d} c {a} d {e} e {b}}
puts [shortestPath $cyclicGraph a e]
puts [shortestPath $cyclicGraph a z]
puts [shortestPath {a {b}} a b]
