pub mod datacentre;
pub mod queueing_model;
pub mod routing;
pub mod service;
pub mod utilisation_model;
pub mod heuristic_model;

use std::collections::{BTreeMap, VecDeque};

use crate::models::service::{Service, ServiceID};
use crate::operators::mapping::{NodeType, RouteNode};

// (ServiceID, Step) -> VnfMetrics
pub type Server = BTreeMap<(usize, usize), VnfMetrics>;

#[derive(Debug, Clone)]
pub struct VnfMetrics {
    pub arrival_rate: f64,
    pub packet_losses: f64,
}

pub fn iterate_route(route: &Vec<RouteNode>, mut apply: impl FnMut(usize)) {
    let mut num_routes: Vec<u32> = route.iter().map(|x| x.route_count).collect();
    let mut queue = VecDeque::new();
    queue.push_back(0);

    while let Some(curr) = queue.pop_front() {
        num_routes[curr] = num_routes[curr] - 1;

        if num_routes[curr] == 0 {
            apply(curr);

            for n in &route[curr].next_nodes {
                queue.push_back(*n);
            }
        }
    }
}

pub fn set_all_arrival_rates(
    solution: &Vec<(ServiceID, Vec<RouteNode>)>,
    services: &Vec<Service>,
    sw_arr: &mut Vec<f64>,
    sw_pl: &Vec<f64>,
    servers: &mut Vec<Server>,
) {
    // Reset memory
    for i in 0..sw_arr.len() {
        sw_arr[i] = 0.0;
    }
    for i in 0..servers.len() {
        for vnf in servers[i].values_mut() {
            vnf.arrival_rate = 0.0;
        }
    }

    let mut num_instances = vec![0; services.len()];
    for (s_id, _) in solution {
        num_instances[*s_id] += 1;
    }

    for (s_id, route) in solution {
        let mut arrs = vec![0.0; route.len()];
        arrs[0] = services[*s_id].prod_rate / num_instances[*s_id] as f64;

        iterate_route(route, |curr| {
            let cn = &route[curr];
            let metrics = get_metrics(cn, *s_id, sw_arr, sw_pl, servers);

            let mut arr = 0.0;
            let mut pl = 0.0;
            if let Some((a, p)) = metrics {
                arr = a;
                pl = p;
            }

            set_arrival_rate(arr + arrs[curr], &cn, *s_id, sw_arr, servers);

            let eff_out = arrs[curr] * (1.0 - pl);
            let distr_out = eff_out / cn.next_nodes.len() as f64;

            for n_id in &cn.next_nodes {
                arrs[*n_id] = arrs[*n_id] + distr_out;
            }
        });
    }
}

fn calc_ma(current_mean: f64, new_value: f64, num_points: usize) -> (f64, f64) {
    let new = current_mean + (new_value - current_mean) / (num_points + 1) as f64;
    (new, (new - current_mean).abs())
}

fn set_all_pl(
    services: &Vec<Service>,
    sw_pl: &mut Vec<f64>,
    sw_arr: &Vec<f64>,
    sw_srv_rate: f64,
    sw_queue_length: usize,
    servers: &mut Vec<Server>,
) {
    for i in 0..sw_pl.len() {
        sw_pl[i] = calc_pl(sw_arr[i], sw_srv_rate, sw_queue_length);
    }

    for i in 0..servers.len() {
        for (&(s_id, pos), vnf_info) in servers[i].iter_mut() {
            // First VNF can't drop packets as it is emitting them
            if pos == 0 {
                continue;
            }

            let vnf = &services[s_id].vnfs[pos];
            vnf_info.packet_losses =
                calc_pl(vnf_info.arrival_rate, vnf.service_rate, vnf.queue_length);
        }
    }
}

fn calc_pl(arrival_rate: f64, service_rate: f64, queue_length: usize) -> f64 {
    let queue_length = queue_length as f64;
    let rho = arrival_rate / service_rate;

    if rho == 1. {
        1. / (queue_length + 1.)
    } else {
        ((1. - rho) * rho.powf(queue_length)) / (1. - rho.powf(queue_length + 1.))
    }
}

fn calc_wt(arrival_rate: f64, service_rate: f64, queue_length: usize, packet_loss: f64) -> f64 {
    let queue_length = queue_length as f64;

    let rho = arrival_rate / service_rate;

    if arrival_rate == 0. {
        return 0.;
    }

    let num_in_system = if rho != 1.0 {
        let a = rho
            * (1.0 - (queue_length + 1.0) * rho.powf(queue_length)
                + queue_length * rho.powf(queue_length + 1.0));
        let b = (1.0 - rho) * (1.0 - rho.powf(queue_length + 1.0));

        a / b
    } else {
        queue_length / 2.0
    };

    let ar = arrival_rate * (1.0 - packet_loss);

    num_in_system / ar
}

fn calc_busy(arrival_rate: f64, service_rate: f64, queue_length: usize) -> f64 {
    if arrival_rate > 0.0 && service_rate == 0.0 {
        return std::f64::INFINITY;
    }

    let rho = arrival_rate / service_rate;
    let k = queue_length as f64;

    let p_empty = if arrival_rate != service_rate {
        (1.0 - rho) / (1.0 - rho.powf(k + 1.0))
    } else {
        1.0 / (k + 1.0)
    };

    1.0 - p_empty
}

pub fn get_metrics(
    rn: &RouteNode,
    service_id: usize,
    sw_arr: &Vec<f64>,
    sw_pl: &Vec<f64>,
    servers: &Vec<Server>,
) -> Option<(f64, f64)> {
    match rn.node_type {
        NodeType::Component(dc_id) => Some((sw_arr[dc_id], sw_pl[dc_id])),
        NodeType::VNF(dc_id, stage) => {
            let vnf = &servers[dc_id].get(&(service_id, stage));

            if let Some(vnf) = vnf {
                Some((vnf.arrival_rate, vnf.packet_losses))
            } else {
                None
            }
        }
    }
}

pub fn set_arrival_rate<'a>(
    arrival_rate: f64,
    rn: &RouteNode,
    service_id: usize,
    sw_arr: &'a mut Vec<f64>,
    servers: &'a mut Vec<Server>,
) {
    match rn.node_type {
        NodeType::Component(dc_id) => sw_arr[dc_id] = arrival_rate,
        NodeType::VNF(dc_id, stage) => {
            let vnf = servers[dc_id]
                .entry((service_id, stage))
                .or_insert(VnfMetrics {
                    arrival_rate: 0.0,
                    packet_losses: 0.0,
                });

            vnf.arrival_rate = arrival_rate;
        }
    }
}

// ----- Unit tests ---- //
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::routing::RoutingTable;
    use crate::models::service::{Service, VNF};
    use crate::operators::mapping::{find_routes, RouteNode};

    #[test]
    fn test_set_arrival_rates() {
        // Simple
        let (simple_route, num_components, num_servers, num_vnfs) = get_simple_route();
        let simple_service = get_service(num_vnfs);
        let mut arr = vec![0.0; num_components];
        let pl = vec![0.0; num_components];
        let mut servers = vec![Server::new(); num_servers];

        set_all_arrival_rates(
            &vec![(0, simple_route)],
            &vec![simple_service],
            &mut arr,
            &pl,
            &mut servers,
        );

        assert_eq!(servers[0][&(0, 0)].arrival_rate, 10.0);
        assert_eq!(arr[0], 10.0);
        assert_eq!(servers[0][&(0, 1)].arrival_rate, 10.0);

        // Branching
        let (branch_route, num_components, num_servers, num_vnfs) = get_branching_route();
        let branching_service = get_service(num_vnfs);
        let mut arr = vec![0.0; num_components];
        let pl = vec![0.0; num_components];
        let mut servers = vec![Server::new(); num_servers];

        set_all_arrival_rates(
            &vec![(0, branch_route)],
            &vec![branching_service],
            &mut arr,
            &pl,
            &mut servers,
        );

        assert_eq!(arr[0], 10.0); // Server 1
        assert_eq!(servers[0][&(0, 0)].arrival_rate, 10.0);
        assert_eq!(arr[1], 10.0); // Server 2
        assert_eq!(servers[1][&(0, 1)].arrival_rate, 10.0);
        assert_eq!(arr[2], 5.0);
        assert_eq!(arr[3], 5.0);
        assert_eq!(arr[4], 5.0 / 3.0);
        assert_eq!(arr[5], 5.0 / 3.0);
        assert_eq!(arr[6], 5.0 / 3.0);
        assert_eq!(arr[6], 5.0 / 3.0);

        // Combined
        let (simple_route, _, _, _) = get_simple_route();
        let (branching_route, num_components, num_servers, num_vnfs) = get_branching_route();

        let simp_service = get_service(num_vnfs);
        let brnc_service = get_service(num_vnfs);

        let mut arr = vec![0.0; num_components];
        let pl = vec![0.0; num_components];
        let mut servers = vec![Server::new(); num_servers];

        set_all_arrival_rates(
            &vec![(0, simple_route), (1, branching_route)],
            &vec![simp_service, brnc_service],
            &mut arr,
            &pl,
            &mut servers,
        );

        assert_eq!(arr[0], 20.0); // Server 1
        assert_eq!(servers[0][&(0, 0)].arrival_rate, 10.0);
        assert_eq!(servers[0][&(0, 1)].arrival_rate, 10.0);
        assert_eq!(servers[0][&(1, 0)].arrival_rate, 10.0);
        assert_eq!(arr[1], 10.0); // Server 2
        assert_eq!(servers[1][&(1, 1)].arrival_rate, 10.0);
        assert_eq!(arr[2], 5.0);
        assert_eq!(arr[3], 5.0);
        assert_eq!(arr[4], 5.0 / 3.0);
        assert_eq!(arr[5], 5.0 / 3.0);
        assert_eq!(arr[6], 5.0 / 3.0);
        assert_eq!(arr[6], 5.0 / 3.0);

        // Lossy Simple
        let (simple_route, num_components, num_servers, num_vnfs) = get_simple_route();
        let simple_service = get_service(num_vnfs);
        let mut arr = vec![0.0; num_components];
        let pl = vec![0.5; num_components];
        let mut servers = vec![Server::new(); num_servers];

        set_all_arrival_rates(
            &vec![(0, simple_route)],
            &vec![simple_service],
            &mut arr,
            &pl,
            &mut servers,
        );

        assert_eq!(servers[0][&(0, 0)].arrival_rate, 10.0);
        assert_eq!(arr[0], 10.0);
        assert_eq!(servers[0][&(0, 1)].arrival_rate, 5.0);
    }

    #[test]
    fn test_ma() {
        let numbers = [
            12.0, 0.2, 20.5, 5.4, 17.0, 0.0, 4.8, 230.2, 2.43, 7.12, 19.2,
        ];
        let mut ma = 0.0;
        for i in 0..numbers.len() {
            let act_mean = &numbers.iter().take(i + 1).sum::<f64>() / (i + 1) as f64;
            ma = calc_ma(ma, numbers[i], i).0;

            assert!((ma - act_mean).abs() < 0.001);
        }
    }

    #[test]
    fn test_iterate_route() {
        let (route, _, _, _) = get_simple_route();
        let expected = vec![0, 1, 2];

        iterate_route(&route, |curr| {
            assert_eq!(curr, expected[curr]);
        });

        let (route, _, _, _) = get_branching_route();
        let expected: Vec<usize> = (0..10).into_iter().collect();
        iterate_route(&route, |curr| {
            assert_eq!(curr, expected[curr]);
        });
    }

    fn get_service(length: usize) -> Service {
        let service = Service {
            id: 0,
            prod_rate: 10.0,
            vnfs: Vec::new(),
        };

        let mut vnfs = Vec::new();
        for _ in 0..length {
            let vnf = VNF {
                size: 100,
                queue_length: 20,
                service_rate: 10.0,
            };

            vnfs.push(vnf);
        }

        service
    }

    // (routes, num_components, num_servers, num_vnfs)
    fn get_simple_route() -> (Vec<RouteNode>, usize, usize, usize) {
        let route = vec![
            RouteNode {
                node_type: NodeType::VNF(0, 0),
                route_count: 1,
                next_nodes: vec![1],
            },
            RouteNode {
                node_type: NodeType::Component(0),
                route_count: 1,
                next_nodes: vec![2],
            },
            RouteNode {
                node_type: NodeType::VNF(0, 1),
                route_count: 1,
                next_nodes: vec![],
            },
        ];

        (route, 1, 1, 2)
    }

    // (routes, num_components, num_servers, num_vnfs)
    fn get_branching_route() -> (Vec<RouteNode>, usize, usize, usize) {
        let route = vec![
            RouteNode {
                node_type: NodeType::VNF(0, 0),
                route_count: 1,
                next_nodes: vec![1],
            },
            RouteNode {
                node_type: NodeType::Component(0),
                route_count: 1,
                next_nodes: vec![2, 3],
            },
            RouteNode {
                node_type: NodeType::Component(2),
                route_count: 1,
                next_nodes: vec![4, 5, 8],
            },
            RouteNode {
                node_type: NodeType::Component(3),
                route_count: 1,
                next_nodes: vec![6, 7, 8],
            },
            RouteNode {
                node_type: NodeType::Component(4),
                route_count: 1,
                next_nodes: vec![8],
            },
            RouteNode {
                node_type: NodeType::Component(5),
                route_count: 1,
                next_nodes: vec![8],
            },
            RouteNode {
                node_type: NodeType::Component(6),
                route_count: 1,
                next_nodes: vec![8],
            },
            RouteNode {
                node_type: NodeType::Component(7),
                route_count: 1,
                next_nodes: vec![8],
            },
            RouteNode {
                node_type: NodeType::Component(1),
                route_count: 6,
                next_nodes: vec![9],
            },
            RouteNode {
                node_type: NodeType::VNF(1, 1),
                route_count: 1,
                next_nodes: vec![],
            },
        ];

        (route, 8, 2, 2)
    }

    fn parse_sim_to_model(
        source: &str,
        routing_tables: &Vec<RoutingTable>,
    ) -> (Vec<Service>, Vec<(usize, Vec<RouteNode>)>) {
        let route_str = source.split(";");

        let mut services = Vec::new();
        let mut routes = Vec::new();

        for (s_id, service) in route_str.enumerate() {
            let sequence: Vec<usize> = service
                .split(",")
                .map(|id| id.parse::<usize>().unwrap())
                .collect();

            let service = get_service(sequence.len());
            services.push(service);

            let route = find_routes(sequence, routing_tables);

            routes.push((s_id, route));
        }

        (services, routes)
    }
}
