use crate::models::datacentre::{Datacentre, NodeID};
use rand::prelude::*;
use std::collections::HashSet;

pub type DistanceMatrix = Vec<Vec<DistanceCell>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DistanceCell {
    pub node_id: NodeID,
    pub distance: usize,
}

pub fn build_cache(graph: &Datacentre, num_nearest: usize) -> DistanceMatrix {
    let num_servers = graph.num_servers;

    // Cache with size num_servers x num_considered
    let mut cache: DistanceMatrix = new_dm(num_servers, num_nearest);

    for start in 0..num_servers {
        set_nearest(&mut cache, graph, start, num_nearest);
    }

    cache
}

pub fn num_samples_upper_bound(
    num_vnfs: usize,
    num_servers: usize,
    p_success_threshold: f64,
) -> usize {
    if num_vnfs > num_servers {
        panic!("Problem is unsolveable in the worst case, the number of VNFs exeeds the number of servers. {} > {}", num_vnfs, num_servers);
    }

    if p_success_threshold < 0.0 || p_success_threshold >= 1.0 {
        panic!("The success probability must be between 0 and 1")
    }

    for i in 1.. {
        let mut prob_placed = 1.0;

        for n in 1..num_vnfs {
            let p_unplaced = (n - 1) as f64 / num_servers as f64;
            prob_placed = prob_placed * (1.0 - p_unplaced.powf(i as f64));

            if prob_placed < p_success_threshold {
                break;
            }
        }

        if prob_placed >= p_success_threshold || i == num_servers {
            return i;
        }
    }

    unreachable!()
}

/**
 * Breadth first search is deterministic which would mean that the same servers would be
 * used repeatedly e.g. in a Fat Tree, the 'left-most' servers would be consistently selected.
 *
 * Instead we randomly select items for expansion from the current horizon. This can be performed
 * efficiently with just two vectors, one for the nodes on the current horizon and one for
 * nodes on the next horizon.  
 */
fn set_nearest(cache: &mut DistanceMatrix, dc: &Datacentre, start: NodeID, num_nearest: usize) {
    // If you know the branching rate of the topology this step can be made more efficient by
    // allocating memory in advance
    let mut current_horizon = vec![start];
    let mut next_horizon = Vec::new();

    let mut visited = HashSet::new();
    visited.insert(start);

    let mut distance = 0;
    let mut num_seen = 0;

    let mut rng = thread_rng();

    while !current_horizon.is_empty() {
        // Choose random node to expand
        let rn = rng.gen_range(0, current_horizon.len());
        let node_id = current_horizon.swap_remove(rn);

        // If we've found a server, add it to the distance matrix
        if dc.is_server(node_id) {
            cache[start].push(DistanceCell { node_id, distance });

            num_seen = num_seen + 1;

            if num_seen >= num_nearest {
                return;
            }
        }

        let neighbours = &dc.graph[node_id];

        // Add neighbours to horizon
        for &neighbour in neighbours {
            if visited.contains(&neighbour) {
                continue;
            }

            next_horizon.push(neighbour);
            visited.insert(neighbour);
        }

        if current_horizon.is_empty() {
            current_horizon = next_horizon;
            next_horizon = Vec::new();
            distance = distance + 1;
        }
    }
}

fn new_dm(num_servers: usize, num_nearest: usize) -> DistanceMatrix {
    vec![Vec::with_capacity(num_nearest); num_servers]
}

// ----- Unit tests ---- //
#[cfg(test)]
mod tests {
    use crate::models::datacentre::FatTree;
    use crate::operators::distance_matrix::*;

    #[test]
    fn test_num_samples_upper_bound() {
        let easiest = num_samples_upper_bound(1, 500, 0.95);
        let expected = num_samples_upper_bound(100, 500, 0.95);
        let hardest = num_samples_upper_bound(500, 500, 0.95);
        let max_success = num_samples_upper_bound(100, 500, 0.99999);

        assert_eq!(easiest, 1);
        assert_eq!(expected, 4);
        assert_eq!(hardest, 500);
        assert_eq!(max_success, 9);
    }

    #[test]
    #[should_panic]
    fn test_nsub_too_many_vnfs() {
        num_samples_upper_bound(501, 500, 0.95);
    }

    #[test]
    fn test_set_nearest() {
        let dc = FatTree::new(4);
        let num_nearest = 6;
        let num_servers = dc.num_servers;

        let mut dm = new_dm(num_servers, num_nearest);

        set_nearest(&mut dm, &dc, 6, num_nearest);

        // Check length
        assert_eq!(dm[6].len(), num_nearest);

        // Check content
        assert_eq!(
            dm[6][0],
            DistanceCell {
                node_id: 6,
                distance: 0
            }
        );
        assert_eq!(
            dm[6][1],
            DistanceCell {
                node_id: 7,
                distance: 2
            }
        );

        assert_eq!(dm[6][2].distance, 4);
        assert_eq!(dm[6][3].distance, 4);
        assert!(vec![4, 5].contains(&dm[6][2].node_id));
        assert!(vec![4, 5].contains(&dm[6][3].node_id));
        assert_ne!(dm[6][2].node_id, dm[6][3].node_id);

        assert_eq!(dm[6][4].distance, 6);
        assert_eq!(dm[6][5].distance, 6);

        // Check randomness
        let mut used_prev = Vec::new();
        for _ in 0..100 {
            dm[6] = vec![];
            set_nearest(&mut dm, &dc, 6, num_nearest);

            let node_id = dm[6][4].node_id;
            assert!(!vec![4, 5, 6, 7].contains(&node_id));
            used_prev.push(node_id);
        }

        used_prev.sort();
        used_prev.dedup();

        assert!(used_prev.len() > 1);
    }

    #[test]
    fn test_build_cache() {
        let dc = FatTree::new(4);
        let num_nearest = 8;
        let num_servers = dc.num_servers;

        let dm = build_cache(&dc, num_nearest);

        // Check length
        assert_eq!(dm.len(), num_servers);

        for i in 0..num_servers {
            assert_eq!(dm[i].len(), num_nearest);
        }

        // Check content
        // Most of this is being checked in 'test_set_nearest'
        for i in 0..num_servers {
            // Self
            assert_eq!(dm[i][0].node_id, i);
            assert_eq!(dm[i][0].distance, 0);
            // Neighbour
            assert_eq!(dm[i][1].distance, 2);
        }
    }
}
