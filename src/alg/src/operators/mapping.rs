use std::collections::{HashMap, VecDeque};

use super::solution::Solution;
use crate::models::datacentre::NodeID;
use crate::models::{routing::RoutingTable, service::Service};
use crate::operators::distance_matrix::DistanceMatrix;
use crate::operators::placement_strategies::NodeSelection;
use rand::{thread_rng, Rng};

pub trait Mapping<X> {
    fn apply(&self, ind: &Solution<X>) -> Vec<(usize, Vec<RouteNode>)>;
}

pub struct IntStringToRouteMapping<'a, X: NodeSelection + Clone> {
    node_selection: X,
    services: &'a Vec<Service>,
    capacities: &'a Vec<usize>,
    distance_matrix: &'a DistanceMatrix,
    routing_tables: &'a Vec<RoutingTable>,
}

impl<'a, X: NodeSelection + Clone> IntStringToRouteMapping<'a, X> {
    pub fn new(
        node_selection: X,
        services: &'a Vec<Service>,
        capacities: &'a Vec<usize>,
        distance_matrix: &'a DistanceMatrix,
        routing_tables: &'a Vec<RoutingTable>,
    ) -> IntStringToRouteMapping<'a, X> {
        IntStringToRouteMapping {
            node_selection,
            services,
            capacities,
            distance_matrix,
            routing_tables,
        }
    }
}

impl<Z: NodeSelection + Clone> Mapping<usize> for IntStringToRouteMapping<'_, Z> {
    fn apply(&self, ind: &Solution<usize>) -> Vec<(usize, Vec<RouteNode>)> {
        let solution_len = self.capacities.len();
        let mut service_string = vec![Vec::new(); solution_len]; // Output

        let mut rng = thread_rng();

        for (i, &num_instances) in ind.point.iter().enumerate() {
            for _ in 0..num_instances {
                let pos = rng.gen_range(0, solution_len);
                service_string[pos].push(&self.services[i]);
            }
        }

        let map_two = ServiceToRouteMapping::new(
            self.node_selection.clone(),
            &self.capacities,
            &self.distance_matrix,
            &self.routing_tables,
        );
        let solution = Solution::new(service_string);

        map_two.apply(&solution)
    }
}

pub struct ServiceToRouteMapping<'a, X: NodeSelection> {
    node_selection: X,
    capacities: &'a Vec<usize>,
    distance_matrix: &'a DistanceMatrix,
    routing_tables: &'a Vec<RoutingTable>,
}

impl<'a, X: NodeSelection> ServiceToRouteMapping<'a, X> {
    pub fn new(
        node_selection: X,
        capacities: &'a Vec<usize>,
        distance_matrix: &'a DistanceMatrix,
        routing_tables: &'a Vec<RoutingTable>,
    ) -> ServiceToRouteMapping<'a, X> {
        ServiceToRouteMapping {
            node_selection,
            capacities,
            distance_matrix,
            routing_tables,
        }
    }
}

impl<Z: NodeSelection> Mapping<Vec<&Service>> for ServiceToRouteMapping<'_, Z> {
    fn apply(&self, ind: &Solution<Vec<&Service>>) -> Vec<(usize, Vec<RouteNode>)> {
        let mut phenotype = vec![]; // Output
        let mut capacities = self.capacities.clone();

        for i in 0..ind.len() {
            let services = &ind[i];

            for service in services {
                let mut pos = i;

                // Find space for sequence of VNFs
                let mut sequence = Vec::with_capacity(service.vnfs.len());
                let mut placed = true;

                for vnf in &service.vnfs {
                    // Apply node selection strategy
                    let row = &self.distance_matrix[pos];
                    let space_at = self.node_selection.select(vnf.size, row, &capacities);

                    if space_at.is_none() {
                        // Insufficient space, free used capacity
                        for i in 0..sequence.len() {
                            let pos = sequence[i];
                            let vnf = &service.vnfs[i];
                            capacities[pos] = capacities[pos] + vnf.size;
                        }
                        placed = false;
                        break;
                    }

                    // Update capacity
                    let curr = space_at.unwrap();
                    capacities[curr] = capacities[curr] - vnf.size;
                    sequence.push(curr);

                    // Repeat for next VNF from new position
                    pos = curr;
                }

                // If everything worked, add it to the solution
                if placed {
                    let routes = find_routes(sequence, &self.routing_tables);
                    phenotype.push((service.id, routes));
                }
            }
        }

        phenotype
    }
}

#[derive(Debug, Clone)]
pub enum NodeType {
    Component(NodeID),
    VNF(NodeID, usize), // (NodeID, Stage)
}

#[derive(Debug, Clone)]
pub struct RouteNode {
    pub node_type: NodeType,
    pub route_count: u32,
    pub next_nodes: Vec<usize>,
}

impl RouteNode {
    fn new_component(dc_node_id: NodeID) -> RouteNode {
        RouteNode {
            node_type: NodeType::Component(dc_node_id),
            route_count: 1,
            next_nodes: Vec::new(),
        }
    }

    fn new_vnf(dc_node_id: NodeID, stage: usize) -> RouteNode {
        RouteNode {
            node_type: NodeType::VNF(dc_node_id, stage),
            route_count: 1,
            next_nodes: Vec::new(),
        }
    }

    pub fn is_vnf(&self) -> bool {
        match self.node_type {
            NodeType::Component(_) => false,
            NodeType::VNF(_, _) => true,
        }
    }

    pub fn dc_id(&self) -> NodeID {
        match self.node_type {
            NodeType::Component(node_id) => node_id,
            NodeType::VNF(node_id, _) => node_id,
        }
    }
}

// Routes are stored in a tree with pointers to nodes in the datacentre
// An adjacency list is used to store the datacentre so the whole route should be loaded
// into memory at a time (in theory)
pub fn find_routes(sequence: Vec<NodeID>, routing_tables: &Vec<RoutingTable>) -> Vec<RouteNode> {
    // Route graph
    let init_server_id = sequence[0];
    let init_node = RouteNode::new_vnf(init_server_id, 0);

    // Multistage graph
    let mut graph: Vec<RouteNode> = vec![init_node];

    // Lookup from DC ID and sequence stage to position in route graph
    // Lookup has to be indexed by stage as the same node can be visited
    // multiple times
    let mut lookup: HashMap<(NodeID, usize), usize> = HashMap::new();

    let mut queue = VecDeque::new();
    queue.push_back((0, graph.len() - 1));

    // Find all the routes between the current node and the target
    while let Some((stage, curr)) = queue.pop_front() {
        // ----- At a VNF
        if let NodeType::VNF(server_id, _) = graph[curr].node_type {
            if stage < sequence.len() - 1 {
                let server = RouteNode::new_component(server_id);
                graph.push(server);

                let next_pos = graph.len() - 1;
                graph[curr].next_nodes.push(next_pos); // Add link to next node
                lookup.insert((server_id, stage), next_pos); // Add next node to lookup

                queue.push_back((stage + 1, next_pos)); // Continue search from next node
            }

            continue;
        }

        // ----- Otherwise, at a server
        let target = sequence[stage];

        // Gives the next set of nodes to visit
        let curr_dc_node = graph[curr].dc_id();
        let next_dc_nodes = routing_tables[curr_dc_node].find(target);

        // --- At the target server
        if curr_dc_node == target {
            let node_id = graph.len();
            graph[curr].next_nodes.push(node_id);

            let vnf = RouteNode::new_vnf(curr_dc_node, stage);
            graph.push(vnf);

            queue.push_back((stage, node_id));

            continue;
        }

        // --- At another server
        for &next_dc_node in next_dc_nodes {
            let lk_next = (next_dc_node, stage);

            if let Some(node_id) = lookup.get(&lk_next) {
                // We have already added this node to the graph
                // increment the route counter on it but don't
                // expand it again
                graph[*node_id].route_count += 1;

                // Point the parent to the right node
                graph[curr].next_nodes.push(*node_id);
            } else {
                // This is the first time we've seen this node
                let node_id = graph.len();
                let component = RouteNode::new_component(next_dc_node);
                graph.push(component);

                lookup.insert(lk_next, node_id);
                queue.push_back((stage, node_id));
                graph[curr].next_nodes.push(node_id);
            }
        }
    }

    graph
}

// ----- Unit tests ---- //
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::datacentre::FatTree;
    use crate::models::routing::get_tables;

    #[test]
    fn test_find_routes() {
        let dc = FatTree::new(4);
        let rts = get_tables(&dc);

        // ------ Repeated
        let sequence = vec![0, 0, 0];
        let graph = find_routes(sequence, &rts);

        assert_eq!(graph.len(), 5);
        test_sequence(
            &graph,
            vec![0, 0, 0, 0, 0],
            vec![true, false, true, false, true],
        );

        // ------ Adjacent
        let sequence = vec![2, 3];
        let graph = find_routes(sequence, &rts);

        assert_eq!(graph.len(), 5);
        test_sequence(
            &graph,
            vec![2, 2, 17, 3, 3],
            vec![true, false, false, false, true],
        );

        // ------ Revisited
        let sequence = vec![4, 5, 4];
        let graph = find_routes(sequence, &rts);

        assert_eq!(graph.len(), 9);
        test_sequence(
            &graph,
            vec![4, 4, 18, 5, 5, 5, 18, 4, 4],
            vec![true, false, false, false, true, false, false, false, true],
        );

        // ------ Split
        let sequence = vec![1, 2];
        let graph = find_routes(sequence, &rts);

        assert_eq!(graph.len(), 8);

        // 1 (VNF)
        let next = test_single(0, 1, true, 1, &graph);

        // 1
        let next = test_single(next, 1, false, 1, &graph);

        // 16
        let next = test_double(next, 16, false, 1, &graph);

        // 24 + 25
        let next_a = test_single(next[0], 24, false, 1, &graph);
        let next_b = test_single(next[1], 25, false, 1, &graph);
        assert_eq!(next_a, next_b);
        let next = next_a;

        // 17
        let next = test_single(next, 17, false, 2, &graph);

        // 2
        let next = test_single(next, 2, false, 1, &graph);

        // 2 (VNF)
        test_last(next, 2, &graph);

        // ----- Big split
        let sequence = vec![0, 15];
        let graph = find_routes(sequence, &rts);

        assert_eq!(graph.len(), 14);

        // 0 (VNF)
        let next = test_single(0, 0, true, 1, &graph);

        // 0
        let next = test_single(next, 0, false, 1, &graph);

        // 16
        let next = test_double(next, 16, false, 1, &graph);

        // 24
        let next_a = test_double(next[0], 24, false, 1, &graph);
        // -- 32
        let next_a_1 = test_single(next_a[0], 32, false, 1, &graph);
        // -- 34
        let next_a_2 = test_single(next_a[1], 34, false, 1, &graph);
        // -- -- 30
        assert_eq!(next_a_1, next_a_2);
        let next_a = test_single(next_a_1, 30, false, 2, &graph);

        // 25
        let next_b = test_double(next[1], 25, false, 1, &graph);
        // -- 33
        let next_b_1 = test_single(next_b[0], 33, false, 1, &graph);
        // -- 35
        let next_b_2 = test_single(next_b[1], 35, false, 1, &graph);
        // -- -- 31
        assert_eq!(next_b_1, next_b_2);
        let next_b = test_single(next_b_1, 31, false, 2, &graph);

        // 24
        assert_eq!(next_a, next_b);
        let next = test_single(next_a, 23, false, 2, &graph);

        // 15
        let next = test_single(next, 15, false, 1, &graph);

        // 15 (VNF)
        test_last(next, 15, &graph);

        // ----- Repeated splits
        let sequence = vec![0, 1, 2];
        let graph = find_routes(sequence, &rts);

        assert_eq!(graph.len(), 12);

        // 0 (VNF)
        let next = test_single(0, 0, true, 1, &graph);

        // 0
        let next = test_single(next, 0, false, 1, &graph);

        // 16
        let next = test_single(next, 16, false, 1, &graph);

        // 1
        let next = test_single(next, 1, false, 1, &graph);

        // 1 (VNF)
        let next = test_single(next, 1, true, 1, &graph);

        // 1
        let next = test_single(next, 1, false, 1, &graph);

        // 16
        let next = test_double(next, 16, false, 1, &graph);

        // -- 24
        let next_a = test_single(next[0], 24, false, 1, &graph);

        // -- 25
        let next_b = test_single(next[1], 25, false, 1, &graph);

        // 17
        assert_eq!(next_a, next_b);
        let next = next_a;
        let next = test_single(next, 17, false, 2, &graph);

        // 2
        let next = test_single(next, 2, false, 1, &graph);

        // 2 (VNF)
        test_last(next, 2, &graph);
    }

    // NOTE: Doesn't work on branching sequences
    fn test_sequence(graph: &Vec<RouteNode>, expected_sequence: Vec<usize>, vnfs: Vec<bool>) {
        let mut next = 0;
        let len = expected_sequence.len();

        for (&elem, is_vnf) in expected_sequence.iter().zip(vnfs).take(len - 1) {
            next = test_single(next, elem, is_vnf, 1, graph);
        }

        test_last(next, expected_sequence[len - 1], &graph);
    }

    fn test_single(
        position: usize,
        node_id: usize,
        is_vnf: bool,
        num_incoming: u32,
        graph: &Vec<RouteNode>,
    ) -> usize {
        assert_eq!(graph[position].is_vnf(), is_vnf);
        assert_eq!(graph[position].dc_id(), node_id);
        assert_eq!(graph[position].next_nodes.len(), 1);
        assert_eq!(graph[position].route_count, num_incoming);
        graph[position].next_nodes[0]
    }

    fn test_double(
        position: usize,
        expected: usize,
        is_vnf: bool,
        num_incoming: u32,
        graph: &Vec<RouteNode>,
    ) -> &Vec<usize> {
        assert_eq!(graph[position].is_vnf(), is_vnf);
        assert_eq!(graph[position].dc_id(), expected);
        assert_eq!(graph[position].next_nodes.len(), 2);
        assert_eq!(graph[position].route_count, num_incoming);

        &graph[position].next_nodes
    }

    fn test_last(position: usize, expected: usize, graph: &Vec<RouteNode>) {
        assert_eq!(graph[position].dc_id(), expected);
        assert!(graph[position].is_vnf());
        assert_eq!(graph[position].next_nodes.len(), 0);
        assert_eq!(graph[position].route_count, 1);
    }
}
