use std::collections::VecDeque;
use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::{models::datacentre::{Datacentre, NodeID}};

/**
 * Gets the routing tables for the provided datacentre.
 **/
pub fn get_tables(datacentre: &Datacentre) -> Vec<RoutingTable> {
    let graph = &datacentre.graph;

    let mut routing_tables = vec![RoutingTable::new(); graph.len()];

    for server in 0..datacentre.num_servers {
        let start = server;
        ecmp(start, graph, |info| {
            routing_tables[info.node].consider(server, info.prev, info.dist)
        });
    }

    routing_tables
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RoutingTable {
    ranges: Vec<Range<usize>>,
    values: Vec<Vec<usize>>,
    min_distance: usize,
}

impl RoutingTable {
    pub fn new() -> RoutingTable {
        RoutingTable {
            ranges: Vec::new(),
            values: Vec::new(),
            min_distance: std::usize::MAX,
        }
    }

    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    pub fn consider(&mut self, server: usize, direction: usize, distance: usize) {
        // Nodes are considered in sequence i.e. 0, 1, 2
        // so we only ever need to consider the last and the second last nodes
        let mut last_idx = self.get_last_idx();
        let same_server = self.ranges.len() > 0 && server == self.ranges[last_idx].end - 1;

        // Only consider the shortest routes for each server
        if same_server && distance > self.min_distance {
            return;
        }

        self.min_distance = distance;

        // Check if we've seen this server before
        if same_server {
            // If we've merged two servers already but we are adding
            // a new element then they must be unmerged
            if self.ranges[last_idx].start != server {
                // Add a new range
                self.ranges.push(Range {
                    start: server,
                    end: server + 1,
                });

                // Copy the previous values
                self.values.push(self.values[last_idx].clone());

                // Update previous range to exclude current server
                self.ranges[last_idx].end -= 1;

                // Update end tracker
                last_idx = last_idx + 1;
            }

            // Add the new direction
            self.values[last_idx].push(direction);
        } else {
            // Add a new row
            self.ranges.push(Range {
                start: server,
                end: server + 1,
            });

            self.values.push(vec![direction]);

            last_idx = last_idx + 1;
        }

        // Check if we can merge the last row and the second last row
        if self.ranges.len() == 1 {
            return;
        }

        if self.values[last_idx] == self.values[last_idx - 1] {
            self.ranges[last_idx - 1].end += 1;

            self.values.pop();
            self.ranges.pop();
        }
    }

    pub fn find(&self, destination: usize) -> &Vec<usize> {
        for i in 0..self.len() {
            let range = &self.ranges[i];
            if destination < range.end {
                return &self.values[i];
            }
        }

        unreachable!();
    }

    fn get_last_idx(&self) -> usize {
        let len = self.ranges.len();

        match len {
            0 => 0,
            1 => 0,
            _ => len - 1,
        }
    }
}

struct Visit {
    pub node: usize,
    pub prev: usize,
    pub dist: usize,
}

fn ecmp(start: usize, graph: &Vec<Vec<NodeID>>, mut func: impl FnMut(Visit)) {
    let mut queue = VecDeque::new();
    let mut visited = vec![];

    func(Visit {
        node: start,
        prev: start,
        dist: 0,
    });

    queue.push_back((0, start));

    while let Some((dist, curr)) = queue.pop_front() {
        if visited.contains(&curr) {
            continue;
        }

        let neighbours = &graph[curr];

        for &neighbour in neighbours {
            if visited.contains(&neighbour) {
                continue;
            }

            func(Visit {
                node: neighbour,
                prev: curr,
                dist,
            });

            queue.push_back((dist + 1, neighbour));
        }

        visited.push(curr);
    }
}

// ----- Unit tests ---- //
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rt_consider() {
        let mut rt_a = RoutingTable::new();
        let mut rt_b = RoutingTable::new();
        let mut rt_c = RoutingTable::new();
        let mut rt_d = RoutingTable::new();

        // Single server
        rt_a.consider(0, 1, 1);
        rt_a.consider(0, 2, 1);
        rt_a.consider(0, 3, 1);
        rt_a.consider(0, 1, 4);
        rt_a.consider(0, 6, 5);

        // Different servers, mergeable
        rt_b.consider(0, 1, 1);
        rt_b.consider(1, 1, 2);
        rt_b.consider(2, 1, 3);
        rt_b.consider(3, 1, 2);

        // Different servers, different distances, mergeable
        rt_c.consider(0, 1, 1);
        rt_c.consider(0, 2, 1);
        rt_c.consider(0, 3, 3);

        rt_c.consider(1, 1, 2);
        rt_c.consider(1, 2, 2);
        rt_c.consider(1, 4, 4);

        rt_c.consider(2, 1, 3);
        rt_c.consider(2, 2, 3);
        rt_c.consider(2, 5, 5);

        rt_c.consider(3, 1, 2);
        rt_c.consider(3, 2, 2);
        rt_c.consider(3, 6, 6);

        // Different servers, must be unmerged
        rt_d.consider(0, 1, 1);
        rt_d.consider(0, 2, 1);
        rt_d.consider(0, 3, 3);

        rt_d.consider(1, 1, 2);
        rt_d.consider(1, 2, 2);
        rt_d.consider(1, 4, 2);

        rt_d.consider(2, 1, 3);
        rt_d.consider(2, 2, 3);
        rt_d.consider(2, 5, 5);

        rt_d.consider(3, 1, 2);
        rt_d.consider(3, 2, 2);
        rt_d.consider(3, 6, 2);

        // Single server
        assert_eq!(rt_a.ranges.len(), rt_a.values.len());
        assert_eq!(rt_a.ranges.len(), 1);

        assert_eq!(rt_a.ranges[0].start, 0);
        assert_eq!(rt_a.ranges[0].end, 1);
        assert_eq!(rt_a.values[0], vec![1, 2, 3]);

        // Different servers, mergeable
        assert_eq!(rt_b.ranges.len(), rt_b.values.len());
        assert_eq!(rt_b.ranges.len(), 1);

        assert_eq!(rt_b.ranges[0].start, 0);
        assert_eq!(rt_b.ranges[0].end, 4);
        assert_eq!(rt_b.values[0], vec![1]);

        // Different servers, different distances, mergeable
        assert_eq!(rt_c.ranges.len(), rt_c.values.len());
        assert_eq!(rt_c.ranges.len(), 1);

        assert_eq!(rt_c.ranges[0].start, 0);
        assert_eq!(rt_c.ranges[0].end, 4);
        assert_eq!(rt_c.values[0], vec![1, 2]);

        // Different servers, must be unmerged
        assert_eq!(rt_d.ranges.len(), rt_d.values.len());
        assert_eq!(rt_d.ranges.len(), 4);

        assert_eq!(rt_d.ranges[0].start, 0);
        assert_eq!(rt_d.ranges[0].end, 1);
        assert_eq!(rt_d.values[0], vec![1, 2]);

        assert_eq!(rt_d.ranges[1].start, 1);
        assert_eq!(rt_d.ranges[1].end, 2);
        assert_eq!(rt_d.values[1], vec![1, 2, 4]);

        assert_eq!(rt_d.ranges[2].start, 2);
        assert_eq!(rt_d.ranges[2].end, 3);
        assert_eq!(rt_d.values[2], vec![1, 2]);

        assert_eq!(rt_d.ranges[3].start, 3);
        assert_eq!(rt_d.ranges[3].end, 4);
        assert_eq!(rt_d.values[3], vec![1, 2, 6]);
    }
}
