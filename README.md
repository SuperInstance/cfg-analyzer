# CFG Analyzer — Control Flow Graph Analysis

**Control Flow Graph (CFG) analysis** is the process of computing structural properties of a program's control flow — reachability, dominance relations, natural loops, and strongly connected components — by modeling basic blocks as nodes and jumps as directed edges.

## Why It Matters

Every optimizing compiler, static analyzer, and program slicer needs a CFG. The analyses in this crate — reachability, dominance, loop detection, SCCs — are the foundation of dead-code elimination, loop-invariant code motion, register allocation, and control-flow integrity checks. Without dominance information, you cannot place φ-functions in SSA form. Without loop detection, you cannot optimize inner loops. These algorithms are the load-bearing steel of compiler middle-ends, binary rewriters, and taint-analysis engines alike.

## How It Works

A **CFG** is a directed graph `G = (V, E)` where each vertex `v ∈ V` is a basic block (a maximal straight-line sequence of instructions with a single entry and single exit) and each edge `(u, v) ∈ E` represents a possible transfer of control (branch, fall-through, call-return). The crate stores each `Block` with its successor and predecessor adjacency lists, enabling both forward and backward traversals.

**Reachability** uses BFS from the entry node: `O(V + E)` time, `O(V)` space. Dead blocks (unreachable from entry) are identified as `V \ reachable(entry)`.

**Dominance** is computed using the iterative Cooper-Harvey-Kennedy algorithm. A node `d` *dominates* `n` if every path from entry to `n` passes through `d`. The *immediate dominator* `idom(n)` is the unique strict dominator of `n` that is dominated by all other strict dominators. The algorithm processes blocks in reverse postorder (RPO), iterating:

```
idom(entry) = entry
for each b in RPO (excluding entry):
    new_idom = first processed predecessor of b
    for each other predecessor p:
        if idom(p) is defined:
            new_idom = intersect(idom, p, new_idom)
    if idom(b) ≠ new_idom: update and mark changed
```

The `intersect` function walks two finger pointers up the dominator tree until they meet, using RPO numbering as a priority. Total complexity: `O(V · D)` where `D` is the maximum depth of the dominator tree — effectively near-linear in practice.

**Natural loops** are detected by finding *back edges* `b → h` where `h` dominates `b`. The loop body is `{h} ∪ {all nodes that can reach b without going through h}`, computed by a backward DFS from `b`. Each natural loop is strongly connected and has a single entry point (the header `h`).

**Strongly Connected Components** use Tarjan's single-DFS algorithm with lowlink tracking: `O(V + E)` time, `O(V)` stack space. Each SCC is either a single non-loop block or a maximal set of mutually reachable blocks.

## Quick Start

```rust
use cfg_analyzer::{CFG, BlockId};

// Build: entry → A → B → C (loop back to A) → exit
let mut cfg = CFG::new(4);
let entry = cfg.add_block(); // 0
let a = cfg.add_block();     // 1
let b = cfg.add_block();     // 2
let exit = cfg.add_block();  // 3

cfg.add_edge(entry, a);
cfg.add_edge(a, b);
cfg.add_edge(b, a);  // back edge: loop A↔B
cfg.add_edge(b, exit);

// Reachability
let reachable = cfg.reachable();
assert_eq!(reachable.len(), 4);

// Dominance
let idom = cfg.immediate_dominators();
// entry dominates everything; idom[a] == entry, idom[b] == a

// Natural loops
let loops = cfg.natural_loops();
assert!(!loops.is_empty()); // {A, B} form a natural loop

// SCCs
let sccs = cfg.sccs();
// {entry}, {A, B}, {exit} — or similar grouping
```

## API

| Type / Function | Description |
|---|---|
| `CFG` | Control flow graph with blocks, entry point, and edge management. |
| `Block` | A basic block: `id`, `successors`, `predecessors`. |
| `BlockId` | Alias for `usize` — indexes into `CFG::blocks`. |
| `CFG::add_block()` | Append a new block, return its `BlockId`. |
| `CFG::add_edge(from, to)` | Add a directed edge (deduped). |
| `CFG::reachable()` | BFS from entry → `HashSet<BlockId>`. |
| `CFG::reverse_postorder()` | RPO traversal — ideal iteration order for dataflow. |
| `CFG::immediate_dominators()` | `Vec<Option<BlockId>>` via iterative algorithm. |
| `CFG::dominance_tree()` | Children-map of the dominator tree. |
| `CFG::natural_loops()` | `Vec<HashSet<BlockId>>` — one set per back edge. |
| `CFG::sccs()` | Tarjan's SCCs: `Vec<Vec<BlockId>>`. |

## Architecture Notes

CFG analysis is a core component of the SuperInstance compilation and analysis pipeline. In the γ + η = C framework (where γ is generation/synthesis and η is evaluation/analysis), this crate serves η — it provides the structural facts that evaluation passes consume. The dominator tree feeds SSA construction; loop detection drives optimization decisions; SCCs identify irreducible control flow. See the [SuperInstance Architecture](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md).

## References

1. Cooper, K. D., Harvey, T. J., & Kennedy, K. (2001). *A Simple, Fast Dominance Algorithm*. Rice University TR. — The iterative algorithm used here.
2. Tarjan, R. (1972). *Depth-First Search and Linear Graph Algorithms*. SIAM J. Comput. 1(2), 146–160. — SCC algorithm.
3. Aho, Lam, Sethi, Ullman (2006). *Compilers: Principles, Techniques, and Tools* (2nd ed.), Ch. 9. — Classic textbook coverage of CFG-based analyses.

## License

MIT
