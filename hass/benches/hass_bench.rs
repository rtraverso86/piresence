use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hass::pirengine::home::{self, VecGraph};

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

    fn g_neighbours(g: &VecGraph<Room>) {
        for n in 0..g.capacity() {
            g.neighbours(n);
        }
    }

    pub fn home_graph_bench(c: &mut Criterion) {
        let mut home = VecGraph::<Room>::new_undirected(20);

        c.bench_function("init", |b| b.iter(|| g_init(black_box(&mut home))));
        c.bench_function("neighbours", |b| b.iter(|| g_neighbours(black_box(&mut home))));
        
    }

}

criterion_group!(benches, graph::home_graph_bench);
criterion_main!(benches);