//! CFG Analyzer - Control Flow Graph analysis utilities.
//!
//! Computes reachability, dominance, loops, and other structural properties.

use std::collections::{HashMap, HashSet, VecDeque};

/// A basic block ID.
pub type BlockId = usize;

/// A basic block in the CFG.
#[derive(Debug, Clone)]
pub struct Block {
    pub id: BlockId,
    pub successors: Vec<BlockId>,
    pub predecessors: Vec<BlockId>,
}

/// A control flow graph.
#[derive(Debug, Clone)]
pub struct CFG {
    pub blocks: Vec<Block>,
    pub entry: BlockId,
}

impl CFG {
    pub fn new(entry_count_hint: usize) -> Self {
        Self { blocks: Vec::with_capacity(entry_count_hint), entry: 0 }
    }

    pub fn add_block(&mut self) -> BlockId {
        let id = self.blocks.len();
        self.blocks.push(Block { id, successors: Vec::new(), predecessors: Vec::new() });
        id
    }

    pub fn add_edge(&mut self, from: BlockId, to: BlockId) {
        if !self.blocks[from].successors.contains(&to) {
            self.blocks[from].successors.push(to);
        }
        if !self.blocks[to].predecessors.contains(&from) {
            self.blocks[to].predecessors.push(from);
        }
    }

    /// BFS reachability from entry.
    pub fn reachable(&self) -> HashSet<BlockId> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(self.entry);
        visited.insert(self.entry);
        while let Some(b) = queue.pop_front() {
            for &s in &self.blocks[b].successors {
                if visited.insert(s) {
                    queue.push_back(s);
                }
            }
        }
        visited
    }

    /// Reverse postorder traversal.
    pub fn reverse_postorder(&self) -> Vec<BlockId> {
        let reachable = self.reachable();
        let mut visited = HashSet::new();
        let mut order = Vec::new();

        fn dfs(cfg: &CFG, b: BlockId, visited: &mut HashSet<BlockId>, order: &mut Vec<BlockId>, reachable: &HashSet<BlockId>) {
            if !visited.insert(b) || !reachable.contains(&b) { return; }
            for &s in &cfg.blocks[b].successors {
                dfs(cfg, s, visited, order, reachable);
            }
            order.push(b);
        }

        dfs(self, self.entry, &mut visited, &mut order, &reachable);
        order.reverse();
        order
    }

    /// Compute immediate dominators using a simple iterative algorithm.
    pub fn immediate_dominators(&self) -> Vec<Option<BlockId>> {
        let reachable = self.reachable();
        let rpo = self.reverse_postorder();
        let n = self.blocks.len();
        let mut idom: Vec<Option<BlockId>> = vec![None; n];
        idom[self.entry] = Some(self.entry);

        let mut changed = true;
        while changed {
            changed = false;
            for &b in &rpo {
                if b == self.entry || !reachable.contains(&b) { continue; }
                let preds: Vec<BlockId> = self.blocks[b].predecessors.iter()
                    .filter(|&&p| reachable.contains(&p) && idom[p].is_some())
                    .copied()
                    .collect();
                if preds.is_empty() { continue; }
                let mut new_idom = preds[0];
                for &p in &preds[1..] {
                    new_idom = self.intersect(&idom, p, new_idom);
                }
                if idom[b] != Some(new_idom) {
                    idom[b] = Some(new_idom);
                    changed = true;
                }
            }
        }
        idom
    }

    fn intersect(&self, idom: &[Option<BlockId>], mut a: BlockId, mut b: BlockId) -> BlockId {
        let mut finger_a = a;
        let mut finger_b = b;
        loop {
            while finger_a > finger_b {
                finger_a = idom[finger_a].unwrap_or(finger_a);
            }
            while finger_b > finger_a {
                finger_b = idom[finger_b].unwrap_or(finger_b);
            }
            if finger_a == finger_b { return finger_a; }
        }
    }

    /// Compute dominance tree (children of each block).
    pub fn dominance_tree(&self) -> HashMap<BlockId, Vec<BlockId>> {
        let idom = self.immediate_dominators();
        let mut tree: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
        for b in 0..self.blocks.len() {
            if let Some(dom) = idom[b] {
                if dom != b {
                    tree.entry(dom).or_default().push(b);
                }
            }
        }
        tree
    }

    /// Find all natural loops (back edges).
    pub fn natural_loops(&self) -> Vec<HashSet<BlockId>> {
        let idom = self.immediate_dominators();
        let mut loops = Vec::new();

        for b in 0..self.blocks.len() {
            for &s in &self.blocks[b].successors {
                // Back edge: b -> s where s dominates b
                if idom.get(b).and_then(|x| *x) == Some(s) || self.dominates(&idom, s, b) {
                    let mut loop_body = HashSet::new();
                    loop_body.insert(s);
                    if b != s {
                        loop_body.insert(b);
                        let mut stack = vec![b];
                        while let Some(n) = stack.pop() {
                            for &p in &self.blocks[n].predecessors {
                                if !loop_body.contains(&p) {
                                    loop_body.insert(p);
                                    stack.push(p);
                                }
                            }
                        }
                    }
                    loops.push(loop_body);
                }
            }
        }
        loops
    }

    fn dominates(&self, idom: &[Option<BlockId>], dom: BlockId, node: BlockId) -> bool {
        let mut current = node;
        loop {
            if current == dom { return true; }
            match idom.get(current).and_then(|x| *x) {
                Some(d) if d != current => current = d,
                _ => return false,
            }
        }
    }

    /// Compute strongly connected components (Tarjan's algorithm).
    pub fn sccs(&self) -> Vec<Vec<BlockId>> {
        let mut index_counter = 0;
        let mut indices: HashMap<BlockId, usize> = HashMap::new();
        let mut lowlinks: HashMap<BlockId, usize> = HashMap::new();
        let mut on_stack: HashSet<BlockId> = HashSet::new();
        let mut stack: Vec<BlockId> = Vec::new();
        let mut sccs: Vec<Vec<BlockId>> = Vec::new();

        for b in 0..self.blocks.len() {
            if !indices.contains_key(&b) {
                self.strongconnect(b, &mut index_counter, &mut indices, &mut lowlinks, &mut on_stack, &mut stack, &mut sccs);
            }
        }
        sccs
    }

    fn strongconnect(
        &self, v: BlockId,
        index_counter: &mut usize,
        indices: &mut HashMap<BlockId, usize>,
        lowlinks: &mut HashMap<BlockId, usize>,
        on_stack: &mut HashSet<BlockId>,
        stack: &mut Vec<BlockId>,
        sccs: &mut Vec<Vec<BlockId>>,
    ) {
        indices.insert(v, *index_counter);
        lowlinks.insert(v, *index_counter);
        *index_counter += 1;
        stack.push(v);
        on_stack.insert(v);

        for &w in &self.blocks[v].successors {
            if !indices.contains_key(&w) {
                self.strongconnect(w, index_counter, indices, lowlinks, on_stack, stack, sccs);
                let vl = *lowlinks.get(&v).unwrap();
                let wl = *lowlinks.get(&w).unwrap();
                lowlinks.insert(v, vl.min(wl));
            } else if on_stack.contains(&w) {
                let vl = *lowlinks.get(&v).unwrap();
                let wi = *indices.get(&w).unwrap();
                lowlinks.insert(v, vl.min(wi));
            }
        }

        if lowlinks[&v] == indices[&v] {
            let mut component = Vec::new();
            loop {
                let w = stack.pop().unwrap();
                on_stack.remove(&w);
                component.push(w);
                if w == v { break; }
            }
            sccs.push(component);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_cfg() {
        let mut cfg = CFG::new(3);
        let b0 = cfg.add_block();
        let b1 = cfg.add_block();
        let b2 = cfg.add_block();
        cfg.add_edge(b0, b1);
        cfg.add_edge(b1, b2);
        assert_eq!(cfg.reachable().len(), 3);
    }

    #[test]
    fn loop_detection() {
        let mut cfg = CFG::new(4);
        let entry = cfg.add_block();
        let header = cfg.add_block();
        let body = cfg.add_block();
        let exit = cfg.add_block();
        cfg.add_edge(entry, header);
        cfg.add_edge(header, body);
        cfg.add_edge(body, header); // back edge
        cfg.add_edge(header, exit);
        let loops = cfg.natural_loops();
        assert!(!loops.is_empty());
    }
}

/// FNV-1a 64 — the digest every substrate in the SuperInstance fleet agrees on.
pub const FNV_OFFSET: u64 = 0xcbf29ce484222325;
pub const FNV_PRIME: u64 = 0x100000001b3;

#[inline]
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h = FNV_OFFSET;
    for &b in bytes {
        h = (h ^ b as u64).wrapping_mul(FNV_PRIME);
    }
    h
}

/// True if this crate's FNV-1a still agrees with the rest of the fleet.
pub fn canary_holds() -> bool {
    fnv1a64("café Δ 日本語".as_bytes()) == 0x024a555471370b18d
}
