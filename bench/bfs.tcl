# Breadth-first search, from sample/bfs.tcl: dicts, list building, foreach, while.
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

    if {[dict exists $graph $node]} {
      foreach toNode [dict get $graph $node] {
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
set cyclicGraph {a {b} b {c d} c {a} d {e} e {b}}

set k 0
while {$k < 2000} {
  shortestPath $graph a f
  shortestPath $cyclicGraph a e
  incr k
}

puts "[shortestPath $graph a f] | [shortestPath $cyclicGraph a e]"
