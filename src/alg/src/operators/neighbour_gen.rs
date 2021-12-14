use rand::prelude::*;

use crate::operators::solution::Solution;

pub trait NeighbourGenerator<X> {
    fn apply(&self, solution: &Solution<X>) -> Solution<X>;
}

#[derive(Clone)]
pub struct AddSwapNeighbour<X> {
    items: Vec<X>,
}

impl<X> AddSwapNeighbour<X> {
    pub fn new(items: Vec<X>) -> AddSwapNeighbour<X> {
        AddSwapNeighbour { items }
    }
}

impl<X: Clone> NeighbourGenerator<Vec<X>> for AddSwapNeighbour<X> {
    fn apply(&self, solution: &Solution<Vec<X>>) -> Solution<Vec<X>> {
        let mut solution = solution.clone();
        let mut rng = rand::thread_rng();

        let rn = rng.gen_range(0.0, 3.0);

        if rn < 1.0 {
            // Add item
            let idx = rng.gen_range(0, solution.len());
            let rnd_item = rng.gen_range(0, self.items.len());

            solution[idx].push(self.items[rnd_item].clone());
        } else if rn < 2.0 {
            // Remove item
            let mut ids: Vec<usize> = (0..solution.len()).collect();
            ids.shuffle(&mut rng);

            for id in ids {
                if !solution[id].is_empty() {
                    let rn = rng.gen_range(0, solution[id].len());
                    solution[id].swap_remove(rn);

                    break;
                }
            }
        } else {
            // Swap server
            let a = rng.gen_range(0, solution.len());
            let b = rng.gen_range(0, solution.len());

            let temp = solution[a].clone();
            solution[a] = solution[b].clone();
            solution[b] = temp;
        }

        solution
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neighbour_gen_mutation() {
        let base_solution = Solution::new(vec![
            vec![0, 1],
            vec![],
            vec![3],
            vec![2, 4, 5],
            vec![],
            vec![],
            vec![],
            vec![6],
            vec![],
            vec![],
            vec![1],
        ]);
        let base_servers = &base_solution.point;

        let ngen = AddSwapNeighbour::new(vec![0, 1, 2, 3, 4, 5, 6]);
        for _ in 0..1000 {
            let new_solution = ngen.apply(&base_solution);

            let mut total_vms = 0;
            let mut servers_changed = 0;

            let servers = &new_solution.point;
            for i in 0..servers.len() {
                total_vms += servers[i].len();

                if servers[i].len() != base_servers[i].len() {
                    servers_changed += 1;
                }
            }

            assert!(total_vms == 7 || total_vms == 8 || total_vms == 9);
            assert!(servers_changed <= 2);
        }
    }
}
