use crate::models::datacentre::NodeID;
use crate::operators::distance_matrix::DistanceCell;

pub trait NodeSelection {
    fn select(
        &self,
        req_capacity: usize,
        row: &Vec<DistanceCell>,
        capacities: &Vec<usize>,
    ) -> Option<NodeID>;
}

#[derive(Clone)]
pub struct FirstFit {}
impl FirstFit {
    pub fn new() -> FirstFit {
        FirstFit {}
    }
}

impl NodeSelection for FirstFit {
    fn select(
        &self,
        req_capacity: usize,
        row: &Vec<DistanceCell>,
        capacities: &Vec<usize>,
    ) -> Option<NodeID> {
        for i in 0..row.len() {
            let node = row[i].node_id;

            // Take the first VNF that fits
            if capacities[node] >= req_capacity {
                return Some(node);
            }
        }

        None
    }
}

#[derive(Clone)]
pub struct BestFit {}
impl BestFit {
    pub fn new() -> BestFit {
        BestFit {}
    }
}

impl NodeSelection for BestFit {
    fn select(
        &self,
        req_capacity: usize,
        row: &Vec<DistanceCell>,
        capacities: &Vec<usize>,
    ) -> Option<NodeID> {
        let mut best_cell: Option<&DistanceCell> = None;
        let mut best_capacity = std::usize::MAX;

        for i in 0..row.len() {
            let cell = &row[i];

            // Only consider the closest nodes
            if let Some(best_cell) = best_cell {
                if best_cell.distance < cell.distance {
                    break;
                }
            }

            // Take the VNF with the least available space
            let capacity = capacities[cell.node_id];
            if capacity < best_capacity && capacity >= req_capacity {
                best_cell = Some(cell);
                best_capacity = capacity;
            }
        }

        best_cell.map(|cell| cell.node_id)
    }
}

pub struct WorstFit {}
impl WorstFit {
    pub fn new() -> WorstFit {
        WorstFit {}
    }
}

impl NodeSelection for WorstFit {
    fn select(
        &self,
        req_capacity: usize,
        row: &Vec<DistanceCell>,
        capacities: &Vec<usize>,
    ) -> Option<NodeID> {
        let mut best_cell: Option<&DistanceCell> = None;
        let mut best_capacity = req_capacity;

        for i in 0..row.len() {
            let cell = &row[i];

            // Only consider the closest nodes
            if let Some(best_cell) = best_cell {
                if best_cell.distance < cell.distance {
                    return Some(best_cell.node_id);
                }
            }

            // Take the VNF with the most available space
            let capacity = capacities[cell.node_id];
            if capacity >= best_capacity {
                best_cell = Some(cell);
                best_capacity = capacity;
            }
        }

        best_cell.map(|cell| cell.node_id)
    }
}

// ----- Unit tests ---- //
#[cfg(test)]
mod tests {
    use super::*;
    use crate::operators::distance_matrix::DistanceCell;

    #[test]
    fn test_first_fit() {
        let (row, capacities) = make_row();

        let selector = FirstFit::new();

        // Finds nearest when possible
        let first = selector.select(20, &row, &capacities);
        assert!(first.is_some());
        assert_eq!(0, first.unwrap());

        // Finds first with capacity when possible
        let nearest = selector.select(40, &row, &capacities);
        assert!(nearest.is_some());
        assert_eq!(2, nearest.unwrap());

        // Finds last if needed
        let last = selector.select(100, &row, &capacities);
        assert!(last.is_some());
        assert_eq!(6, last.unwrap());
    }

    #[test]
    fn test_best_fit() {
        let (row, capacities) = make_row();

        let selector = BestFit::new();
        // Finds nearest when possible
        let first = selector.select(20, &row, &capacities);
        assert!(first.is_some());
        assert_eq!(0, first.unwrap());

        // Finds nearest server with closest capacity #1
        let nearest_one = selector.select(30, &row, &capacities);
        assert!(nearest_one.is_some());
        assert_eq!(1, nearest_one.unwrap());

        // Finds nearest server with closest capacity (boundary check)
        let nearest_two = selector.select(25, &row, &capacities);
        assert!(nearest_two.is_some());
        assert_eq!(1, nearest_two.unwrap());

        // Finds last if needed
        let last = selector.select(100, &row, &capacities);
        assert!(last.is_some());
        assert_eq!(6, last.unwrap());
    }

    #[test]
    fn test_worst_fit() {
        let (row, capacities) = make_row();

        let selector = WorstFit::new();

        // Finds nearest when possible
        let first = selector.select(20, &row, &capacities);
        assert!(first.is_some());
        assert_eq!(0, first.unwrap());

        // Finds nearest server with worst capacity #1
        let nearest_one = selector.select(30, &row, &capacities);
        assert!(nearest_one.is_some());
        assert_eq!(2, nearest_one.unwrap());

        // Finds nearest server with closest capacity (boundary check)
        let nearest_two = selector.select(55, &row, &capacities);
        assert!(nearest_two.is_some());
        assert_eq!(5, nearest_two.unwrap());

        // Finds last if needed
        let last = selector.select(100, &row, &capacities);
        assert!(last.is_some());
        assert_eq!(6, last.unwrap());
    }

    fn make_row() -> (Vec<DistanceCell>, Vec<usize>) {
        let row = vec![
            DistanceCell {
                node_id: 0,
                distance: 0,
            },
            DistanceCell {
                node_id: 1,
                distance: 1,
            },
            DistanceCell {
                node_id: 2,
                distance: 1,
            },
            DistanceCell {
                node_id: 3,
                distance: 1,
            },
            DistanceCell {
                node_id: 4,
                distance: 2,
            },
            DistanceCell {
                node_id: 5,
                distance: 2,
            },
            DistanceCell {
                node_id: 6,
                distance: 3,
            },
        ];

        let capacities = vec![20, 30, 50, 40, 25, 60, 100];

        (row, capacities)
    }
}
