use crate::models::datacentre::Datacentre;
use crate::models::service::{Service, ServiceID};
use crate::models::{
    calc_busy, calc_ma, calc_wt, get_metrics, iterate_route, set_all_arrival_rates, set_all_pl,
    Server, VnfMetrics,
};
use crate::operators::mapping::{NodeType, RouteNode};

// As the model is run very frequently and the DBs are quite large
// we cache the memory to prevent lots of mallocs
#[derive(Clone)]
pub struct QueueingModel<'a> {
    dc: &'a Datacentre,
    sw_sr: f64,
    sw_ql: usize,
    pub target_acc: f64,
    pub converged_iterations: usize,
    active_cost: f64,
    idle_cost: f64,
}

impl<'a> QueueingModel<'a> {
    pub fn new(
        dc: &Datacentre,
        sw_sr: f64,
        sw_ql: usize,
        accuracy: f64,
        converged_iterations: usize,
        active_cost: f64,
        idle_cost: f64,
    ) -> QueueingModel {
        QueueingModel {
            dc,
            sw_sr,
            sw_ql,
            target_acc: accuracy,
            converged_iterations,
            active_cost,
            idle_cost,
        }
    }

    pub fn evaluate(
        &self,
        services: &Vec<Service>,
        routes: &Vec<(ServiceID, Vec<RouteNode>)>,
    ) -> (Vec<f64>, Vec<f64>, f64) {
        let mut servers_mean: Vec<Server> = vec![Server::new(); self.dc.num_servers];
        let mut servers_temp: Vec<Server> = vec![Server::new(); self.dc.num_servers];

        let mut sw_arr_mean = vec![0.0; self.dc.num_components()];
        let mut sw_arr_temp = vec![0.0; self.dc.num_components()];
        let mut sw_pl = vec![0.0; self.dc.num_components()];

        // Reset the entries and warm up the cache
        for i in 0..self.dc.num_servers {
            servers_mean[i].clear();
            servers_temp[i].clear();
        }

        for i in 0..self.dc.num_components() {
            sw_arr_mean[i] = 0.0;
            sw_arr_temp[i] = 0.0;
            sw_pl[i] = 0.0;
        }

        // Calculate arrival rate
        let mut num_iterations = 0;
        let mut num_below = 0;
        let mut max_diff: f64;

        while num_below < self.converged_iterations {
            // Add arrival rates
            set_all_arrival_rates(
                &routes,
                &services,
                &mut sw_arr_temp,
                &sw_pl,
                &mut servers_temp,
            );

            // Calculate packet loss for all components
            set_all_pl(
                &services,
                &mut sw_pl, // Switch info
                &sw_arr_temp,
                self.sw_sr,
                self.sw_ql,
                &mut servers_temp,
            );

            // Cumulative moving average of arrival rates
            max_diff = 0.0;

            for i in 0..sw_arr_temp.len() {
                let (new, diff) = calc_ma(sw_arr_mean[i], sw_arr_temp[i], num_iterations);
                sw_arr_mean[i] = new;

                max_diff = max_diff.max(diff);
            }

            for i in 0..servers_temp.len() {
                for (&(s_id, pos), met) in &servers_temp[i] {
                    let temp = met.arrival_rate;

                    let vnf_info = servers_mean[i].entry((s_id, pos)).or_insert(VnfMetrics {
                        arrival_rate: 0.0,
                        packet_losses: 0.0,
                    });

                    let (new, diff) = calc_ma(vnf_info.arrival_rate, temp, num_iterations);
                    vnf_info.arrival_rate = new;

                    max_diff = max_diff.max(diff);
                }
            }

            if max_diff < self.target_acc {
                num_below = num_below + 1;
            } else {
                num_below = 0;
            }

            num_iterations = num_iterations + 1;
        }

        // Recalculate PL using average arrival rate
        set_all_pl(
            &services,
            &mut sw_pl, // Switch info
            &sw_arr_mean,
            self.sw_sr,
            self.sw_ql,
            &mut servers_mean,
        );

        // Calculate service latency + pl
        let mut service_latency = vec![0.0; services.len()];
        let mut service_pl = vec![0.0; services.len()];

        let mut s_count = vec![0; services.len()];

        for (s_id, route) in routes {
            let mut node_pk = vec![0.0; route.len()]; // Probability a packet survives to this node
            let mut node_pl = vec![0.0; route.len()]; // Packet loss at this node
            let mut node_pv = vec![0.0; route.len()]; // Probability of visiting this node
            node_pv[0] = 1.0;
            node_pk[0] = 1.0;

            iterate_route(route, |curr| {
                let (_, pl) =
                    get_metrics(&route[curr], *s_id, &sw_arr_mean, &sw_pl, &mut servers_mean)
                        .unwrap();

                node_pl[curr] = pl;
                node_pk[curr] = node_pk[curr] * (1.0 - node_pl[curr]);

                let num_next = route[curr].next_nodes.len();
                if num_next == 0 {
                    service_pl[*s_id] =
                        calc_ma(service_pl[*s_id], 1.0 - node_pk[curr], s_count[*s_id]).0;
                }

                for node in &route[curr].next_nodes {
                    node_pk[*node] += node_pk[curr] / num_next as f64;
                    node_pv[*node] += node_pv[curr] / num_next as f64;
                }
            });

            let mut latency = 0.0;
            for i in 1..route.len() {
                let rn = &route[i];
                let (arr, _) =
                    get_metrics(rn, *s_id, &sw_arr_mean, &sw_pl, &mut servers_mean).unwrap();

                let (srv, ql) = match rn.node_type {
                    NodeType::Component(_) => (self.sw_sr, self.sw_ql),
                    NodeType::VNF(_, stage) => {
                        let vnf = &services[*s_id].vnfs[stage];
                        (vnf.service_rate, vnf.queue_length)
                    }
                };

                latency = latency + (calc_wt(arr, srv, ql, node_pl[i]) * node_pv[i]);
            }

            service_latency[*s_id] = calc_ma(service_latency[*s_id], latency, s_count[*s_id]).0;
            s_count[*s_id] += 1;
        }

        // Calculate energy consumption
        let energy = self.get_energy_consumption(
            services,
            &servers_mean,
            &sw_arr_mean,
            self.sw_sr,
            self.sw_ql,
        );

        (service_latency, service_pl, energy)
    }

    pub fn get_energy_consumption(
        &self,
        services: &Vec<Service>,
        servers_mean: &Vec<Server>,
        sw_arr_mean: &Vec<f64>,
        sw_sr: f64,
        sw_ql: usize,
    ) -> f64 {
        let mut sum_energy = 0.0;

        for i in 0..self.dc.num_components() {
            let utilisation;
            if self.dc.is_server(i) {
                let server_busy = calc_busy(sw_arr_mean[i], sw_sr, sw_ql);
                let mut p_none_busy = 1.0;

                for (&(s_id, pos), vnf) in &servers_mean[i] {
                    // Producing VNFs don't go towards energy consumption
                    if pos == 0 {
                        continue;
                    }

                    let vnf_info = &services[s_id].vnfs[pos];

                    let vm_not_busy = 1.0
                        - calc_busy(
                            vnf.arrival_rate,
                            vnf_info.service_rate,
                            vnf_info.queue_length,
                        );
                    p_none_busy = p_none_busy * vm_not_busy;
                }

                utilisation = 1.0 - ((1.0 - server_busy) * p_none_busy)
            } else {
                utilisation = calc_busy(sw_arr_mean[i], self.sw_sr, self.sw_ql)
            };

            if utilisation == 0.0 {
                continue;
            }

            sum_energy += (self.active_cost * utilisation) + (self.idle_cost * (1.0 - utilisation));
        }

        sum_energy
    }
}
