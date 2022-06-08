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
}

impl<N> VecGraph<N> {
    pub fn new_undirected(capacity: usize) -> VecGraph<N> {
        VecGraph {
            edges: vec![false; capacity * capacity],
            nodes: Vec::with_capacity(capacity),
            capacity,
        }
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
        let (idx1, idx2) = self.get_edge_id(from, to);
        self.edges[idx1] = true;
        self.edges[idx2] = true;
    }

    pub fn remove_edge(&mut self, from: NodeId, to: NodeId) {
        let (idx1, idx2) = self.get_edge_id(from, to);
        self.edges[idx1] = false;
        self.edges[idx2] = false;
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    #[inline]
    fn get_edge_id(&self, from: NodeId, to: NodeId) -> (usize, usize) {
        let sz = self.nodes.len();
        if sz == 0 || from.max(to) >= sz {
            panic!("unexisting nodes");
        }
        (from * sz + to, to * sz + from)
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


#[cfg(test)]
mod tests {
    use super::*;

    struct Room {
        pub id: String,
    }

    impl Room {
        pub fn new(id: &str) -> Room {
            Room { id: id.to_owned() }
        }
    }

    #[test]
    pub fn undirected_small() {
        let mut home = VecGraph::<Room>::new_undirected(3);
        assert_eq!(home.capacity(), 3);
        assert_eq!(home.node_count(), 0);
        let id_entrance = home.add_node(Room::new("entrance")).unwrap();
        assert_eq!(home.node_count(), 1);
        let id_living = home.add_node(Room::new("living room")).unwrap();
        assert_eq!(home.node_count(), 2);
        let id_kitchen = home.add_node(Room::new("kitchen")).unwrap();
        assert_eq!(home.node_count(), 3);
        home.add_edge(id_entrance, id_living);
        home.add_edge(id_living, id_kitchen);

        assert_eq!(home.capacity(), 3);
        assert_eq!(home.node_count(), 3);

        let found_living = home.find_node_id(|r| { r.id == "living room"}).unwrap();
        assert_eq!(found_living, id_living);

        assert!(home.find_node_id(|_| {false} ).is_none());

        let expected_neighbours = vec![id_living];
        assert_eq!(expected_neighbours, home.neighbours(id_entrance));

        let expected_neighbours = vec![id_entrance, id_kitchen];
        assert_eq!(expected_neighbours, home.neighbours(id_living));

        let expected_neighbours = vec![id_living];
        assert_eq!(expected_neighbours, home.neighbours(id_kitchen));
    }
}