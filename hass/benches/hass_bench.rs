use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hass::pirengine::home::{self, VecGraph, Graph};

mod graph {
    use rand::Rng;

    use super::*;

    struct Room {
        pub id: String,
    }

    fn g_init(g: &mut VecGraph<Room>) {
        g.clear();
        let capacity = g.capacity();
        for n in 0..capacity {
            g.add_node(Room {id: format!("Room #{}", n).to_owned()});
        }
        let gen_edge = || {
            rand::thread_rng().gen_range(0..capacity)
        };
        for _ in 0..(capacity*2) {
            let edge1 = gen_edge();
            let edge2 = gen_edge();
            g.add_edge(edge1, edge2);
        }
    }

    fn g_neighbours(g: &mut VecGraph<Room>) {
        for n in 0..g.capacity() {
            g.neighbours(n);
        }
    }

    fn g_get_node(g: &VecGraph<Room>) {
        for n in 0..g.capacity() {
            g.get_node(n);
        }
    }

    pub fn home_graph_bench(c: &mut Criterion) {
        let mut home = VecGraph::<Room>::new_undirected(20);

        c.bench_function("init", |b| b.iter(|| g_init(black_box(&mut home))));
        c.bench_function("neighbours", |b| b.iter(|| g_neighbours(black_box(&mut home))));
        c.bench_function("get_node", |b| b.iter(|| g_get_node(black_box(&home))));
        
    }

}

mod trie {
    use std::collections::HashMap;

    use super::*;
    use hass::pirengine::trie::{HashedTrie, HashMapTrie, TrieLookup, Trie};
    use rand::{Rng, RngCore};

    pub fn insert_benchmark(c: &mut Criterion) {
        let mut h_trie = HashedTrie::new();
        let mut hm_htrie = HashMapTrie::new();
        let mut trie = Trie::new();

        let mut rng = rand::thread_rng();
        let mut keys: Vec<String> = Vec::with_capacity(1000);
        for _ in 0..1000 {
            let key: String = (0..10).map(|_| (0x20u8 + (rng.next_u32() % 96) as u8) as char).collect();
            keys.push(key);
        }

        let mut group = c.benchmark_group("TrieLookup::insert()");
        group.bench_function("HashedTrie", |b| {
            b.iter(|| {
                for i in 0..1000 {
                    h_trie.insert(&keys[i], i as i32);
                }
            })
        });
        group.bench_function("HashMapTrie", |b| {
            b.iter(|| {
                for i in 0..1000 {
                    hm_htrie.insert(&keys[i], i as i32);
                }
            })
        });
        group.bench_function("Trie", |b| {
            b.iter(|| {
                for i in 0..1000 {
                    trie.insert(&keys[i], i as i32);
                }
            })
        });
    }

    pub fn search_benchmark(c: &mut Criterion) {
        let mut h_trie = HashedTrie::new();
        let mut hm_trie = HashMapTrie::new();
        let mut trie = Trie::new();

        let mut rng = rand::thread_rng();
        let mut keys: Vec<String> = Vec::with_capacity(1000);
        for i in 0..1000 {
            let key: String = (0..10).map(|_| (0x20u8 + (rng.next_u32() % 96) as u8) as char).collect();
            keys.push(key);
            h_trie.insert(&keys[i], i as i32);
            hm_trie.insert(&keys[i], i as i32);
            trie.insert(&keys[i], i as i32)
        }

        let mut group = c.benchmark_group("TrieLookup::search()");
        group.bench_function("HashedTrie", |b| {
            b.iter(|| {
                for i in 0..1000 {
                    black_box(h_trie.search(&keys[i]));
                }
            })
        });
        group.bench_function("HashMapTrie", |b| {
            b.iter(|| {
                for i in 0..1000 {
                    black_box(hm_trie.search(&keys[i]));
                }
            })
        });
        group.bench_function("Trie", |b| {
            b.iter(|| {
                for i in 0..1000 {
                    black_box(trie.search(&keys[i]));
                }
            })
        });
    }
}

criterion_group!(benches_graph, graph::home_graph_bench);
criterion_group!(benches_trie, trie::insert_benchmark, trie::search_benchmark);
criterion_main!(benches_graph, benches_trie);