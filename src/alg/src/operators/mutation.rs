use rand::prelude::*;

use crate::operators::solution::Solution;

pub trait Mutation<X> {
    fn apply(&self, solution: &Solution<X>) -> Solution<X>;
}

#[derive(Clone)]
pub struct SwapMutation {
    pm: f64,
    mr: f64, // Mutation rate
}

impl SwapMutation {
    pub fn new(pm: f64, mr: f64) -> SwapMutation {
        SwapMutation { pm, mr }
    }
}

impl<X: Clone> Mutation<X> for SwapMutation {
    fn apply(&self, solution: &Solution<X>) -> Solution<X> {
        let mut solution = solution.clone();

        let mut rng = rand::thread_rng();
        if rng.gen::<f64>() > self.pm {
            return solution;
        }

        for i in 0..solution.len() {
            if rng.gen::<f64>() > self.mr {
                continue;
            }

            let swap = rng.gen_range(0, solution.len());

            let temp = solution.point[i].clone();
            solution[i] = solution[swap].clone();
            solution[swap] = temp;
        }

        solution
    }
}

#[derive(Clone)]
pub struct AddRemoveMutation<X> {
    pm: f64,
    mr: f64, // Mutation rate
    items: Vec<X>,
}

impl<X> AddRemoveMutation<X> {
    pub fn new(items: Vec<X>, pm: f64, mr: f64) -> AddRemoveMutation<X> {
        AddRemoveMutation { items, pm, mr }
    }
}

/**
    Mutation designed specifically for strings of vectors.
    Adds or removes a random item to the vector of the current character.
**/
impl<X: Clone> Mutation<Vec<X>> for AddRemoveMutation<X> {
    fn apply(&self, solution: &Solution<Vec<X>>) -> Solution<Vec<X>> {
        let mut solution = solution.clone();

        let mut rng = rand::thread_rng();

        if rng.gen::<f64>() > self.pm {
            return solution;
        }

        for i in 0..solution.len() {
            if rng.gen::<f64>() > self.mr {
                continue;
            }

            if rng.gen_bool(0.5) {
                // Add a character
                let c_idx = rng.gen_range(0, self.items.len());
                let item = self.items[c_idx].clone();

                solution[i].push(item);
            } else {
                if solution[i].len() == 0 {
                    continue;
                }

                // Remove a random item
                let item_idx = rng.gen_range(0, solution[i].len());
                solution[i].swap_remove(item_idx);
            }
        }

        solution
    }
}

#[derive(Clone)]
pub struct SingleExchange {}

impl SingleExchange {
    pub fn new() -> SingleExchange {
        SingleExchange {}
    }
}

impl<X: Clone> Mutation<Vec<X>> for SingleExchange {
    fn apply(&self, solution: &Solution<Vec<X>>) -> Solution<Vec<X>> {
        let mut solution = solution.clone();

        let mut rng = thread_rng();

        let a = rng.gen_range(0, solution.len());
        let b = rng.gen_range(0, solution.len());

        let temp = solution.point[a].clone();
        solution[a] = solution[b].clone();
        solution[b] = temp;

        solution
    }
}

pub struct IncrDecrMutation {
    pm: f64,
    mr: f64,
}

impl IncrDecrMutation {
    pub fn new(pm: f64, mr: f64) -> IncrDecrMutation {
        IncrDecrMutation { pm, mr }
    }
}

impl Mutation<usize> for IncrDecrMutation {
    fn apply(&self, solution: &Solution<usize>) -> Solution<usize> {
        let mut solution = solution.clone();

        let mut rng = rand::thread_rng();
        if rng.gen::<f64>() > self.pm {
            return solution;
        }

        for i in 0..solution.len() {
            if rng.gen::<f64>() > self.mr {
                continue;
            }

            if rng.gen() {
                solution[i] += 1;
            } else {
                solution[i] -= 1;
            }
        }

        solution
    }
}

#[derive(Clone)]
pub struct AddRemoveSwapMutation<X> {
    pm: f64,
    items: Vec<X>,
}

impl<X> AddRemoveSwapMutation<X> {
    pub fn new(items: Vec<X>, pm: f64) -> AddRemoveSwapMutation<X> {
        AddRemoveSwapMutation { items, pm }
    }
}

impl<X: Clone> Mutation<Vec<X>> for AddRemoveSwapMutation<X> {
    fn apply(&self, solution: &Solution<Vec<X>>) -> Solution<Vec<X>> {
        let mut solution = solution.clone();
        let mut rng = rand::thread_rng();

        if self.pm >= rng.gen_range(0.0, 1.0) {
            return solution;
        }

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
    fn test_swap_mutation() {
        let ind_len = 100;
        let point: Vec<usize> = (0..ind_len).into_iter().collect();
        let ind = Solution::new(point);

        let mutation = SwapMutation::new(1.0, 1.0);
        let mut new_ind = mutation.apply(&ind);

        assert_eq!(new_ind.len(), ind_len);
        assert_ne!(new_ind.point, ind.point);

        new_ind.point.sort();
        assert_eq!(new_ind.point, ind.point);
    }

    #[test]
    fn test_add_remove_mutation() {
        let num_items = 5;
        let items: Vec<usize> = (0..num_items).into_iter().collect();

        let mutation = AddRemoveMutation::new(items, 1.0, 1.0);

        let ind_len = 1000;

        // Empty case
        let point = vec![vec![]; ind_len];
        let empty_ind = Solution::new(point);
        let empty_ind = mutation.apply(&empty_ind);

        // Full case
        let point = vec![vec![1]; ind_len];
        let full_ind = Solution::new(point);
        let full_ind = mutation.apply(&full_ind);

        // Check length
        assert_eq!(empty_ind.len(), ind_len);
        assert_eq!(full_ind.len(), ind_len);

        // Check distribution
        // -- In the empty case we'd expect ~50% of cells to have an item after mutation
        let num_items = empty_ind
            .point
            .into_iter()
            .fold(0, |acc, cell| acc + if cell.is_empty() { 0 } else { 1 });

        assert!(num_items < 550 && num_items > 450);

        // -- In the full case we'd expect ~50% of cells to be empty after mutation
        let num_cells = full_ind
            .point
            .into_iter()
            .fold(0, |acc, cell| acc + if cell.is_empty() { 1 } else { 0 });

        assert!(num_cells < 550 && num_cells > 450);
    }
}
