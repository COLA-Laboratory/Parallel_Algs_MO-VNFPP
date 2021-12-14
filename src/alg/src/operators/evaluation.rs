use super::{
    distance_matrix::DistanceMatrix, mapping::RouteNode, placement_strategies::NodeSelection,
    solution::Constraint,
};
use crate::{
    models::{
        datacentre::Datacentre, heuristic_model::HeuristicModel, queueing_model::QueueingModel,
        routing::RoutingTable, service::Service, utilisation_model::UtilisationModel,
    },
    utilities::metrics::mean,
};

pub trait Evaluation {
    fn evaluate_ind(&self, routes: &Vec<(usize, Vec<RouteNode>)>) -> Constraint<Vec<f64>, usize>;
}

// --- Queueing Model
#[derive(Clone)]
pub struct QueueingEval<'a, N: NodeSelection> {
    capacities: &'a Vec<usize>,
    distance_matrix: &'a DistanceMatrix,
    pub queueing_model: QueueingModel<'a>,
    routing_tables: &'a Vec<RoutingTable>,
    services: &'a Vec<Service>,
    node_selection: N,
    pub use_hf_cnstr: bool,
}

impl<'a, N: NodeSelection> QueueingEval<'a, N> {
    pub fn new(
        queueing_model: QueueingModel<'a>,
        routing_tables: &'a Vec<RoutingTable>,
        distance_matrix: &'a DistanceMatrix,
        capacities: &'a Vec<usize>,
        services: &'a Vec<Service>,
        node_selection: N,
    ) -> QueueingEval<'a, N> {
        QueueingEval {
            capacities,
            distance_matrix,
            queueing_model,
            routing_tables,
            services,
            node_selection,
            use_hf_cnstr: true,
        }
    }
}

impl<NS: NodeSelection> Evaluation for QueueingEval<'_, NS> {
    fn evaluate_ind(&self, routes: &Vec<(usize, Vec<RouteNode>)>) -> Constraint<Vec<f64>, usize> {
        let num_unplaced = num_unplaced(&routes, &self.services);
        if num_unplaced > 0 {
            return if self.use_hf_cnstr {
                Constraint::Infeasible(num_unplaced)
            } else {
                Constraint::Infeasible(0)
            };
        }

        let (latencies, pls, energy) = self.queueing_model.evaluate(&self.services, &routes);

        let avg_latency = mean(&latencies);
        let avg_pl = mean(&pls);

        Constraint::Feasible(vec![avg_latency, avg_pl, energy])
    }
}

// --- Utilisation Model
pub struct UtilisationEval<'a, N: NodeSelection> {
    capacities: Vec<usize>,
    distance_matrix: &'a DistanceMatrix,
    util_model: UtilisationModel<'a>,
    routing_tables: &'a Vec<RoutingTable>,
    services: &'a Vec<Service>,
    node_selection: &'a N,
}

impl<'a, N: NodeSelection> UtilisationEval<'a, N> {
    pub fn new(
        dc: &Datacentre,
        routing_tables: &'a Vec<RoutingTable>,
        distance_matrix: &'a DistanceMatrix,
        capacities: Vec<usize>,
        services: &'a Vec<Service>,
        sw_sr: f64,
        sw_ql: usize,
        node_selection: &'a N,
        queueing_model: QueueingModel<'a>,
    ) -> UtilisationEval<'a, N> {
        let util_model = UtilisationModel::new(dc, queueing_model, sw_sr, sw_ql);

        UtilisationEval {
            capacities,
            distance_matrix,
            util_model,
            routing_tables,
            services,
            node_selection,
        }
    }
}

impl<NS: NodeSelection> Evaluation for UtilisationEval<'_, NS> {
    fn evaluate_ind(&self, routes: &Vec<(usize, Vec<RouteNode>)>) -> Constraint<Vec<f64>, usize> {
        // -- Evaluate solution and check feasibility
        let num_unplaced = num_unplaced(&routes, &self.services);
        if num_unplaced > 0 {
            return Constraint::Infeasible(num_unplaced);
        }

        let (service_utilisation, energy) =
            self.util_model
                .evaluate(&self.services, &routes, |util| util);

        let avg_util = mean(&service_utilisation);

        Constraint::Feasible(vec![avg_util, energy])
    }
}

// --- Heuristic Model
pub struct HeuristicEval<'a, N: NodeSelection> {
    routing_tables: &'a Vec<RoutingTable>,
    distance_matrix: &'a DistanceMatrix,
    capacities: Vec<usize>,
    heuristic_model: HeuristicModel<'a>,
    services: &'a Vec<Service>,
    node_selection: &'a N,
}

impl<'a, N: NodeSelection> HeuristicEval<'a, N> {
    pub fn new(
        dc: &'a Datacentre,
        routing_tables: &'a Vec<RoutingTable>,
        distance_matrix: &'a DistanceMatrix,
        capacities: Vec<usize>,
        services: &'a Vec<Service>,
        node_selection: &'a N,
    ) -> HeuristicEval<'a, N> {
        let heuristic_model = HeuristicModel::new(dc);

        HeuristicEval {
            routing_tables,
            distance_matrix,
            capacities,
            heuristic_model,
            services,
            node_selection,
        }
    }
}

impl<NS: NodeSelection> Evaluation for HeuristicEval<'_, NS> {
    fn evaluate_ind(&self, routes: &Vec<(usize, Vec<RouteNode>)>) -> Constraint<Vec<f64>, usize> {
        // -- Evaluate solution and check feasibility
        let num_unplaced = num_unplaced(&routes, &self.services);
        if num_unplaced > 0 {
            return Constraint::Infeasible(num_unplaced);
        }

        let (perc_used, len) = self.heuristic_model.evaluate(&routes);

        Constraint::Feasible(vec![perc_used, len])
    }
}

// --- Helpers
pub fn num_unplaced(routes: &Vec<(usize, Vec<RouteNode>)>, services: &Vec<Service>) -> usize {
    let mut counts = vec![0; services.len()];
    routes.iter().for_each(|&(s_id, _)| {
        counts[s_id] += 1;
    });

    counts.iter().filter(|&&count| count == 0).count()
}
