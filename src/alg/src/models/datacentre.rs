use serde::{Deserialize, Serialize};
use std::fmt::Display;

pub type NodeID = usize;

#[derive(Copy, Clone)]
pub enum Topology {
    FatTree,
    LeafSpine,
    DCell,
}

impl Display for Topology {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Topology::DCell => write!(f, "DCell"),
            Topology::FatTree => write!(f, "FatTree"),
            Topology::LeafSpine => write!(f, "LeafSpine"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Datacentre {
    pub graph: Vec<Vec<NodeID>>,
    pub num_ports: usize,
    pub num_servers: usize,
}

impl Datacentre {
    pub fn is_server(&self, node_id: usize) -> bool {
        node_id < self.num_servers
    }

    pub fn num_components(&self) -> usize {
        self.graph.len()
    }
}

pub struct FatTree;
impl FatTree {
    pub fn new(num_ports: usize) -> Datacentre {
        let num_servers = num_ports.pow(3) / 4;
        let num_edges = num_ports * (num_ports / 2);
        let num_agg = num_edges;
        let num_core = (num_ports / 2).pow(2);

        let num_nodes = num_servers + num_edges + num_agg + num_core;

        let mut graph = vec![Vec::new(); num_nodes];

        let edg_off = num_servers;
        let agg_off = num_servers + num_edges;
        let cor_off = num_servers + num_edges + num_agg;

        for i in 0..num_servers {
            let edge_id = i / (num_ports / 2); // usize rounds towards zero
            graph[i].push(edge_id + num_servers);
        }

        for i in 0..num_edges {
            // connect servers
            let server_min = i * (num_ports / 2);

            for j in server_min..server_min + (num_ports / 2) {
                graph[i + edg_off].push(j);
            }

            // connect aggregate switches
            let agg_min = (i / (num_ports / 2)) * (num_ports / 2);
            for j in agg_min..agg_min + (num_ports / 2) {
                graph[i + edg_off].push(j + agg_off);
            }
        }

        for i in 0..num_agg {
            // connect edge switches
            let edge_min = (i / (num_ports / 2)) * (num_ports / 2);
            for j in edge_min..edge_min + (num_ports / 2) {
                graph[i + agg_off].push(j + edg_off);
            }

            // connect core switches
            let mut j = i % (num_ports / 2);
            let max = (num_ports / 2).pow(2);
            while j < max {
                graph[i + agg_off].push(j + cor_off);
                j = j + num_ports / 2;
            }
        }

        for i in 0..num_core {
            // connect aggregate switches
            let mut j = i % (num_ports / 2);
            let max = num_agg;

            while j < max {
                graph[i + cor_off].push(j + agg_off);
                j = j + num_ports / 2;
            }
        }

        Datacentre {
            graph,
            num_ports,
            num_servers,
        }
    }
}

pub struct LeafSpine;
impl LeafSpine {
    pub fn new(num_ports: usize, num_spine: usize) -> Datacentre {
        let num_leaf = num_ports;
        let num_servers = (num_ports - num_spine) * num_leaf;

        let num_nodes = num_spine + num_leaf + num_servers;

        let mut graph = vec![Vec::new(); num_nodes];

        let leaf_off = num_servers;
        let spin_off = num_servers + num_leaf;

        for i in 0..num_servers {
            // connect leaf nodes
            let leaf_id = i / (num_ports - num_spine); // usize rounds towards zero
            graph[i].push(leaf_id + leaf_off);
        }

        for i in 0..num_leaf {
            // connect servers
            let server_min = i * (num_ports - num_spine);

            for j in server_min..server_min + (num_ports - num_spine) {
                graph[i + leaf_off].push(j);
            }

            // connect spine
            for j in 0..num_spine {
                graph[i + leaf_off].push(j + spin_off);
            }
        }

        for i in 0..num_spine {
            // connect leaf
            for j in 0..num_leaf {
                graph[i + spin_off].push(j + leaf_off);
            }
        }

        Datacentre {
            graph,
            num_ports,
            num_servers,
        }
    }
}

pub struct DCell;
impl DCell {
    pub fn new(num_ports: usize, level: usize) -> Datacentre {
        let num_servers = DCell::num_servers(level, num_ports);
        let num_switches = num_servers / num_ports;
        let num_nodes = num_servers + num_switches;

        let mut graph = vec![Vec::new(); num_nodes];
        DCell::build_dcells(&mut graph, Vec::new(), num_ports, level, num_servers);

        Datacentre {
            graph,
            num_ports,
            num_servers,
        }
    }

    fn build_dcells(
        graph: &mut Vec<Vec<NodeID>>,
        prefix: Vec<usize>,
        num_ports: usize,
        level: usize,
        sw_offset: usize,
    ) {
        if level == 0 {
            for i in 0..num_ports {
                let mut new_prefix = prefix.clone();
                new_prefix.push(i);

                let srv_id = DCell::to_id(&new_prefix, num_ports);
                let sw_id = srv_id / num_ports;

                graph[srv_id].push(sw_offset + sw_id);
                graph[sw_offset + sw_id].push(srv_id);
            }

            return;
        }

        let gl = DCell::num_dcells(level, num_ports);

        for i in 0..gl {
            let mut new_prefix = prefix.clone();
            new_prefix.push(i);

            DCell::build_dcells(graph, new_prefix, num_ports, level - 1, sw_offset);
        }

        let tl = DCell::num_servers(level - 1, num_ports);

        for i in 0..tl {
            for j in i + 1..gl {
                let mut uid_a = DCell::to_tuple(j - 1, num_ports, level - 1);
                let mut uid_b = DCell::to_tuple(i, num_ports, level - 1);

                let mut n_a = prefix.clone();
                n_a.push(i);
                n_a.append(&mut uid_a);

                let mut n_b = prefix.clone();
                n_b.push(j);
                n_b.append(&mut uid_b);

                let id_a = DCell::to_id(&n_a, num_ports);
                let id_b = DCell::to_id(&n_b, num_ports);

                graph[id_a].push(id_b);
                graph[id_b].push(id_a);
            }
        }

        return;
    }

    fn num_dcells(level: usize, num_ports: usize) -> usize {
        match level {
            0 => 1,
            _ => DCell::num_servers(level - 1, num_ports) + 1,
        }
    }

    fn num_servers(level: usize, num_ports: usize) -> usize {
        match level {
            0 => num_ports,
            _ => DCell::num_dcells(level, num_ports) * DCell::num_servers(level - 1, num_ports),
        }
    }

    fn to_id(prefix: &Vec<usize>, num_ports: usize) -> usize {
        let mut flip_prefix = prefix.clone();
        flip_prefix.reverse();

        let mut sum = flip_prefix[0];

        for i in 1..flip_prefix.len() {
            sum = sum + flip_prefix[i] * DCell::num_servers(i - 1, num_ports);
        }

        sum
    }

    fn to_tuple(uid: usize, num_ports: usize, level: usize) -> Vec<usize> {
        let mut num_servers = vec![num_ports; level + 1];
        for i in 0..level + 1 {
            num_servers[i] = DCell::num_servers(i, num_ports);
        }
        num_servers.reverse();

        let mut tuple = vec![0; level + 1];
        let mut uid = uid;

        for i in 0..level + 1 {
            if i == level {
                tuple[i] = uid % num_servers[i];
            } else {
                tuple[i] = uid / num_servers[i + 1];
                uid = uid % num_servers[i + 1];
            }
        }

        tuple
    }
}
