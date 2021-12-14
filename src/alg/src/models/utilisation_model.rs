use super::queueing_model::QueueingModel;
use crate::models::datacentre::Datacentre;
use crate::models::service::{Service, ServiceID};
use crate::models::{get_metrics, iterate_route, set_all_arrival_rates, Server};
use crate::operators::mapping::{NodeType, RouteNode};

pub struct UtilisationModel<'a> {
    qm: QueueingModel<'a>,
    sw_sr: f64,
    sw_ql: usize,
    num_components: usize,
    num_servers: usize,
}

impl<'a> UtilisationModel<'a> {
    pub fn new(
        dc: &Datacentre,
        qm: QueueingModel<'a>,
        sw_sr: f64,
        sw_ql: usize,
    ) -> UtilisationModel<'a> {
        UtilisationModel {
            qm,
            sw_sr,
            sw_ql,
            num_components: dc.num_components(),
            num_servers: dc.num_servers,
        }
    }

    pub fn evaluate<F>(
        &self,
        services: &Vec<Service>,
        routes: &Vec<(ServiceID, Vec<RouteNode>)>,
        modifier: F,
    ) -> (Vec<f64>, f64)
    where
        F: Fn(f64) -> f64,
    {
        let mut sw_arr = vec![0.0; self.num_components];
        let mut sw_pl = vec![0.0; self.num_components];
        let mut servers = vec![Server::new(); self.num_servers];

        let mut expected_utils = vec![1.0; services.len()];

        // Reset the entries and warm up the cache
        for i in 0..sw_arr.len() {
            sw_arr[i] = 0.0;
            sw_pl[i] = 0.0;
        }
        for i in 0..servers.len() {
            servers[i].clear();
        }

        set_all_arrival_rates(routes, services, &mut sw_arr, &sw_pl, &mut servers);

        for (s_id, route) in routes {
            let mut node_ev = vec![0.0; route.len()]; // Expected number of visits to this node
            node_ev[0] = 1.0;
            let mut node_util = vec![0.0; route.len()];

            iterate_route(route, |curr| {
                let cn = &route[curr];

                // Utilisation of each node
                let (arr, _) = get_metrics(cn, *s_id, &sw_arr, &sw_pl, &servers).unwrap();

                let sr = match cn.node_type {
                    NodeType::Component(_) => self.sw_sr,
                    NodeType::VNF(_, stage) => services[*s_id].vnfs[stage].service_rate,
                };

                node_util[curr] = modifier(arr / sr);

                // Probability of visiting each node
                let num_next = route[curr].next_nodes.len();
                for node in &route[curr].next_nodes {
                    node_ev[*node] += node_ev[curr] / num_next as f64;
                }
            });

            // Calculate expected utilisation
            let mut expected_utilisation = 0.0;
            for i in 0..route.len() {
                expected_utilisation += node_util[i]; // * node_ev[i];
            }

            expected_utils[*s_id] = expected_utilisation;
        }

        let energy = self
            .qm
            .get_energy_consumption(services, &servers, &sw_arr, self.sw_sr, self.sw_ql);

        (expected_utils, energy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{datacentre::FatTree, routing, service::VNF},
        operators::mapping::find_routes,
    };

    #[test]
    pub fn test_evaluate() {
        let fat_tree = FatTree::new(4);
        let rt = routing::get_tables(&fat_tree);

        let sw_sr = 1.0;
        let sw_ql = 10;

        let qm = QueueingModel::new(&fat_tree, sw_sr, sw_ql, 500.0, 0, 1.0, 1.0);

        let util_model = UtilisationModel::new(&fat_tree, qm, sw_sr, sw_ql);

        let services = Service {
            id: 0,
            prod_rate: 1.0,
            vnfs: vec![
                VNF {
                    queue_length: sw_ql,
                    service_rate: 1.0,
                    size: 100,
                },
                VNF {
                    queue_length: sw_ql,
                    service_rate: 1.0,
                    size: 100,
                },
                VNF {
                    queue_length: sw_ql,
                    service_rate: 1.0,
                    size: 100,
                },
            ],
        };

        let simple_seq = vec![0, 1, 2];
        let simple_rts = find_routes(simple_seq, &rt);

        let branching_seq = vec![0, 15, 1];
        let branching_rts = find_routes(branching_seq, &rt);

        let mod_rts = vec![0, 1, 2];
        let mod_rts = find_routes(mod_rts, &rt);

        let (simp_util, simp_energy) =
            util_model.evaluate(&vec![services.clone()], &vec![(0, simple_rts)], |f| f);

        assert_eq!(simp_util[0], 14.5);
        assert_eq!(simp_energy, 7.0);

        let (branch_util, branch_energy) =
            util_model.evaluate(&vec![services.clone()], &vec![(0, branching_rts)], |f| f);

        assert_eq!(branch_util[0], 22.0);
        assert_eq!(branch_energy, 13.0);

        let (mod_util, mod_energy) =
            util_model.evaluate(&vec![services.clone()], &vec![(0, mod_rts)], |f| f * 2.0);

        assert_eq!(mod_util[0], 29.0);
        assert_eq!(mod_energy, 7.0);
    }
}
