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

pub trait Graph<N> {
    fn new_undirected(capacity: usize) -> Self;
    fn clear(&mut self);
    fn add_node(&mut self, node: N) -> Option<NodeId>;
    fn find_node_id(&self, predicate: Box<dyn Fn(&N) -> bool>) -> Option<NodeId>;
    fn get_node(&self, node_id: NodeId) -> &N;
    fn add_edge(&mut self, from: NodeId, to: NodeId);
    fn remove_edge(&mut self, from: NodeId, to:NodeId);
    fn capacity(&self) -> usize;
    fn node_count(&self) -> usize;
    fn neighbours(&mut self, of: NodeId) -> &Vec<NodeId>;
}

pub struct VecGraph<N> {
    edges: Vec<bool>,
    nodes: Vec<N>,
    capacity: usize,
    _neigh: Option<Vec<NodeId>>,
}

impl<N> VecGraph<N> {
    #[inline]
    fn get_edge_id(&self, from: NodeId, to: NodeId) -> (usize, usize) {
        let sz = self.nodes.len();
        if sz == 0 || from.max(to) >= sz {
            panic!("unexisting nodes");
        }
        (from * sz + to, to * sz + from)
    }
}

impl<N> Graph<N> for VecGraph<N> {
    fn new_undirected(capacity: usize) -> VecGraph<N> {
        VecGraph {
            edges: vec![false; capacity * capacity],
            nodes: Vec::with_capacity(capacity),
            capacity,
            _neigh: None,
        }
    }

    fn clear(&mut self) {
        self.nodes.clear();
        for ele in &mut self.edges {
            *ele = false;
        }
    }

    fn add_node(&mut self, node: N) -> Option<NodeId> {
        if self.nodes.len() == self.nodes.capacity() {
            return None;
        }
        self.nodes.push(node);
        Some(self.nodes.len() - 1)
    }

    fn find_node_id(&self, predicate: Box<dyn Fn(&N) -> bool>) -> Option<NodeId> {
        self.nodes.iter()
            .enumerate()
            .find(|x| predicate(x.1))
            .map(|n| n.0)
    }

    fn get_node(&self, node_id: NodeId) -> &N {
        &self.nodes[node_id as usize]
    }

    fn add_edge(&mut self, from: NodeId, to: NodeId) {
        let (idx1, idx2) = self.get_edge_id(from, to);
        self.edges[idx1] = true;
        self.edges[idx2] = true;
    }

    fn remove_edge(&mut self, from: NodeId, to: NodeId) {
        let (idx1, idx2) = self.get_edge_id(from, to);
        self.edges[idx1] = false;
        self.edges[idx2] = false;
    }

    fn capacity(&self) -> usize {
        self.capacity
    }

    fn node_count(&self) -> usize {
        self.nodes.len()
    }

    fn neighbours(&mut self, of: NodeId) -> &Vec<NodeId> {
        //if  self._neigh.is_none() {
            let mut ns = vec![];
            let sz = self.nodes.len();
            let base = of * sz;
            for (i, idx) in (base..(base + sz)).enumerate() {
                if self.edges[idx] && i != of {
                    ns.push(i);
                }
            }
            self._neigh = Some(ns);
        //}
        let ns = self._neigh.as_ref().unwrap();
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

        let found_living = home.find_node_id(Box::new(|r| { r.id == "living room"})).unwrap();
        assert_eq!(found_living, id_living);

        assert!(home.find_node_id(Box::new(|_| {false}) ).is_none());

        let expected_neighbours = vec![id_living];
        assert_eq!(&expected_neighbours, home.neighbours(id_entrance));

        let expected_neighbours = vec![id_entrance, id_kitchen];
        assert_eq!(&expected_neighbours, home.neighbours(id_living));

        let expected_neighbours = vec![id_living];
        assert_eq!(&expected_neighbours, home.neighbours(id_kitchen));

        home.remove_edge(id_living, id_entrance);

        assert!(home.neighbours(id_entrance).is_empty());

        let expected_neighbours = vec![id_kitchen];
        assert_eq!(&expected_neighbours, home.neighbours(id_living));

        home.remove_edge(id_living, id_kitchen);

        assert!(home.neighbours(id_living).is_empty());

        assert!(home.neighbours(id_kitchen).is_empty());
    }
}