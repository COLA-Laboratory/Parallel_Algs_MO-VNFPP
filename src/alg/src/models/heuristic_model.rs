use super::{datacentre::Datacentre, service::ServiceID};
use crate::operators::mapping::{NodeType, RouteNode};
use std::collections::HashSet;

pub struct HeuristicModel<'a> {
    dc: &'a Datacentre,
}

impl<'a> HeuristicModel<'a> {
    pub fn new(dc: &'a Datacentre) -> HeuristicModel<'a> {
        HeuristicModel { dc }
    }

    pub fn evaluate(
        &self,
        // placements: &Solution<Vec<&Service>>,
        routes: &Vec<(ServiceID, Vec<RouteNode>)>,
    ) -> (f64, f64) {
        // Count number of distinct components
        let mut ids = HashSet::new();

        for (_, route) in routes {
            for node in route {
                if let NodeType::Component(id) = node.node_type {
                    ids.insert(id);
                }
            }
        }

        let perc_used = ids.len() as f64 / self.dc.num_components() as f64;

        let sum_len = routes.iter().map(|(_, route)| route.len()).sum::<usize>();
        let avg_len: f64 = sum_len as f64 / routes.len() as f64;

        (perc_used, avg_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::datacentre::FatTree;

    #[test]
    pub fn test_evaluate() {
        let dc = FatTree::new(4);
        let mut model = HeuristicModel::new(&dc);

        println!("{}", dc.num_components());

        let mut simple_route = Vec::new();
        simple_route.push((
            0,
            vec![
                route_node(NodeType::VNF(0, 0)),
                route_node(NodeType::Component(0)),
                route_node(NodeType::Component(1)),
                route_node(NodeType::Component(2)),
                route_node(NodeType::Component(3)),
                route_node(NodeType::VNF(1, 1)),
            ],
        ));

        let mut revisited_route = Vec::new();
        revisited_route.push((
            0,
            vec![
                route_node(NodeType::VNF(0, 0)),
                route_node(NodeType::Component(0)),
                route_node(NodeType::Component(1)),
                route_node(NodeType::Component(2)),
                route_node(NodeType::Component(3)),
                route_node(NodeType::VNF(1, 1)),
                route_node(NodeType::Component(1)),
                route_node(NodeType::Component(3)),
                route_node(NodeType::Component(5)),
                route_node(NodeType::VNF(0, 1)),
            ],
        ));

        let mut multiple_routes = Vec::new();
        multiple_routes.push((
            0,
            vec![
                route_node(NodeType::VNF(0, 0)),
                route_node(NodeType::Component(0)),
                route_node(NodeType::Component(1)),
                route_node(NodeType::Component(2)),
                route_node(NodeType::Component(3)),
                route_node(NodeType::VNF(1, 1)),
            ],
        ));
        multiple_routes.push((
            1,
            vec![
                route_node(NodeType::VNF(0, 0)),
                route_node(NodeType::Component(0)),
                route_node(NodeType::Component(1)),
                route_node(NodeType::Component(2)),
                route_node(NodeType::Component(3)),
                route_node(NodeType::Component(4)),
                route_node(NodeType::Component(5)),
                route_node(NodeType::VNF(1, 1)),
            ],
        ));

        let (perc_used_simp, avg_len_simp) = model.evaluate(&simple_route);
        let (perc_used_rev, avg_len_rev) = model.evaluate(&revisited_route);
        let (perc_used_mult, avg_len_mult) = model.evaluate(&multiple_routes);

        assert_eq!(perc_used_simp, 4.0 / 36.0);
        assert_eq!(avg_len_simp, 6.0);

        assert_eq!(perc_used_rev, 5.0 / 36.0);
        assert_eq!(avg_len_rev, 10.0);

        assert_eq!(perc_used_mult, 6.0 / 36.0);
        assert_eq!(avg_len_mult, 7.0);
    }

    fn route_node(rn: NodeType) -> RouteNode {
        RouteNode {
            node_type: rn,
            route_count: 1,
            next_nodes: Vec::new(),
        }
    }
}
