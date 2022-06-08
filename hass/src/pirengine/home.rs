//! Home Representation
//!
//! This module holds data structures to represent rooms/areas part of
//! the home, how they're connected to each other, and which smart devices
//! they do contain.

pub type AreaId = String;

pub enum Presence {
    NoOne,
    AtLeast(u8),
    AtMost(u8),
}

pub struct Area {
    id: String,
    pub presence_esimate: Presence,
}

impl Area {
    pub fn new(id: &str) -> Area {
        Area {
            id: id.to_owned(),
            presence_esimate: Presence::NoOne,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

pub type NodeId = usize;

pub struct VecGraph<N> {
    edges: Vec<bool>,
    nodes: Vec<N>,
    capacity: usize,
    is_undirected: bool,
}

impl<N> VecGraph<N> {
    pub fn new(capacity: usize, is_undirected: bool) -> VecGraph<N> {
        VecGraph {
            edges: vec![false; capacity * capacity],
            nodes: Vec::with_capacity(capacity),
            capacity,
            is_undirected,
        }
    }

    pub fn new_directed(capacity: usize) -> VecGraph<N> {
        Self::new(capacity, false)
    }

    pub fn new_undirected(capacity: usize) -> VecGraph<N> {
        Self::new(capacity, true)
    }

    pub fn add_node(&mut self, node: N) -> Option<NodeId> {
        if self.nodes.len() == self.nodes.capacity() {
            return None;
        }
        self.nodes.push(node);
        Some(self.nodes.len() - 1)
    }

    pub fn find_node_id<P>(&self, predicate: P) -> Option<NodeId>
    where
        P: Fn(&N) -> bool
    {
        self.nodes.iter()
            .enumerate()
            .find(|x| predicate(x.1))
            .map(|n| n.0)
    }

    pub fn get_node(&self, node_id: NodeId) -> &N {
        &self.nodes[node_id as usize]
    }

    pub fn add_edge(&mut self, from: NodeId, to: NodeId) {
        let idx = self.get_edge_id(from, to);
        self.edges[idx] = true;
        if self.is_undirected {
            let idx = self.get_edge_id(to, from);
            self.edges[idx] = true;
        }
    }

    pub fn remove_edge(&mut self, from: NodeId, to: NodeId) {
        let idx = self.get_edge_id(from, to);
        self.edges[idx] = false;
        if self.is_undirected {
            let idx = self.get_edge_id(to, from);
            self.edges[idx] = false;
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    #[inline]
    fn get_edge_id(&self, from: NodeId, to: NodeId) -> usize {
        let sz = self.nodes.len();
        if sz == 0 || from.max(to) >= sz {
            panic!("unexisting nodes");
        }
        from * sz + to
    }

    pub fn neighbours(&self, of: NodeId) -> Vec<NodeId> {
        let mut ns = vec![];
        let sz = self.nodes.len();
        let base = of * sz;
        for (i, idx) in (base..(base + sz)).enumerate() {
            if self.edges[idx] && i != of {
                ns.push(i);
            }
        }
        ns
    }

}
