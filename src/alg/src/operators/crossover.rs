use rand::prelude::*;

use crate::operators::distance_matrix::DistanceMatrix;
use crate::operators::solution::Solution;

pub trait Crossover<X> {
    fn apply(&self, parent_one: &Solution<X>, parent_two: &Solution<X>) -> Vec<Solution<X>>;
}

/*** Uniform Crossover ***/
pub struct UniformCrossover {
    pc: f64,
}

impl UniformCrossover {
    pub fn new(pc: f64) -> UniformCrossover {
        UniformCrossover { pc }
    }
}

impl<X: Clone> Crossover<X> for UniformCrossover {
    fn apply(&self, parent_one: &Solution<X>, parent_two: &Solution<X>) -> Vec<Solution<X>> {
        let mut rng = rand::thread_rng();

        if rng.gen::<f64>() > self.pc {
            return vec![parent_one.clone(), parent_two.clone()];
        }

        let length = parent_one.len();

        let mut child_a = Vec::with_capacity(length);
        let mut child_b = Vec::with_capacity(length);

        for i in 0..length {
            if random() {
                child_a.push(parent_one[i].clone());
                child_b.push(parent_two[i].clone());
            } else {
                child_a.push(parent_two[i].clone());
                child_b.push(parent_one[i].clone());
            }
        }

        let child_a = Solution::new(child_a);
        let child_b = Solution::new(child_b);

        vec![child_a, child_b]
    }
}

/*** N-point crossover ***/
pub struct NPointCrossover {
    pc: f64,
    num_cuts: usize,
}

impl NPointCrossover {
    pub fn new(pc: f64, num_cuts: usize) -> NPointCrossover {
        NPointCrossover { pc, num_cuts }
    }
}

impl<X: Clone> Crossover<X> for NPointCrossover {
    fn apply(&self, parent_one: &Solution<X>, parent_two: &Solution<X>) -> Vec<Solution<X>> {
        if self.num_cuts > parent_one.len() {
            panic!("The number of crossover points cannot exceed the number of variables");
        }

        let solution_len = parent_one.len();

        let mut rng = rand::thread_rng();

        if rng.gen::<f64>() > self.pc {
            return vec![parent_one.clone(), parent_two.clone()];
        }

        // Choose random split locations and sort them
        let mut split_locations = Vec::with_capacity(self.num_cuts);
        let mut rng = thread_rng();
        while split_locations.len() < self.num_cuts {
            let split_loc = rng.gen_range(0, solution_len);
            if split_locations.contains(&split_loc) {
                continue;
            }
            split_locations.push(split_loc);
        }
        split_locations.sort_unstable();

        // Create new solutionss
        let len = parent_one.len();
        let mut child_a = Vec::with_capacity(len);
        let mut child_b = Vec::with_capacity(len);

        let mut curr_split = 0;
        let mut match_parent = true;

        for i in 0..len {
            if curr_split < split_locations.len() && split_locations[curr_split] == i {
                match_parent = !match_parent;
                curr_split = curr_split + 1;
            }
            if match_parent {
                child_a.push(parent_one[i].clone());
                child_b.push(parent_two[i].clone());
            } else {
                child_a.push(parent_two[i].clone());
                child_b.push(parent_one[i].clone());
            }
        }

        let child_a = Solution::new(child_a);
        let child_b = Solution::new(child_b);

        vec![child_a, child_b]
    }
}

/**
 * Local exchange crossover
 * The crossover operator we propose in the paper.
 *
 * Performs a crossover whilst maintaining the relative placement of services
 * on nearby servers.
 */
pub struct LocalExchange<'a> {
    pc: f64,
    perc_exchange: f64,
    dm: &'a DistanceMatrix,
}

impl LocalExchange<'_> {
    pub fn new(pc: f64, prop_exchange: f64, dm: &DistanceMatrix) -> LocalExchange {
        if dm.len() == 0 {
            panic!("The distance matrix is empty.");
        }

        if prop_exchange > 1.0 || prop_exchange <= 0.0 {
            panic!("Percentage to exchange must be between (0, 1].")
        }

        LocalExchange {
            pc,
            perc_exchange: prop_exchange,
            dm,
        }
    }
}

impl<X: Clone> Crossover<X> for LocalExchange<'_> {
    fn apply(&self, parent_one: &Solution<X>, parent_two: &Solution<X>) -> Vec<Solution<X>> {
        let mut rng = rand::thread_rng();

        if rng.gen::<f64>() > self.pc {
            return vec![parent_one.clone(), parent_two.clone()];
        }

        let mut rng = rand::thread_rng();
        if rng.gen::<f64>() > self.pc {
            return vec![parent_one.clone(), parent_two.clone()];
        }

        let solution_len = parent_one.len();

        // Exchange neighbours until half have been exchanged
        let mut child_a = parent_one.clone();
        let mut child_b = parent_two.clone();

        let mut marked = vec![false; solution_len];
        let mut num_marked = 0;

        loop {
            // Choose random neighbour
            let pos = rng.gen_range(0, solution_len);

            let num_dm = (self.dm[pos].len() as f64 * self.perc_exchange) as usize;
            let num_dm = num_dm.max(1); // Ensure we exchange at least one

            for i in 0..num_dm {
                let neighbour = self.dm[pos][i].node_id;

                if marked[neighbour] {
                    continue;
                }

                child_a[neighbour] = parent_two[neighbour].clone();
                child_b[neighbour] = parent_one[neighbour].clone();

                marked[neighbour] = true;
                num_marked += 1;

                if num_marked == (solution_len / 2) {
                    return vec![child_a, child_b];
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::models::datacentre::FatTree;
    use crate::operators::distance_matrix::build_cache;

    #[test]
    fn test_uniform_crossover() {
        let crossover_func = UniformCrossover::new(1.0);
        basic_tests(crossover_func, 50);
    }

    #[test]
    fn test_n_point_crossover() {
        let crossover_func_1 = NPointCrossover::new(1.0, 1);
        let crossover_func_2 = NPointCrossover::new(1.0, 2);
        let crossover_func_3 = NPointCrossover::new(1.0, 3);
        let crossover_func_4 = NPointCrossover::new(1.0, 4);
        let crossover_func_5 = NPointCrossover::new(1.0, 5);

        basic_tests(crossover_func_1, 10);
        basic_tests(crossover_func_2, 10);
        basic_tests(crossover_func_3, 10);
        basic_tests(crossover_func_4, 10);
        basic_tests(crossover_func_5, 10);
    }

    #[test]
    #[should_panic(
        expected = "The number of crossover points cannot exceed the number of variables"
    )]
    fn test_n_point_crossover_many_points() {
        let crossover_func = NPointCrossover::new(1.0, 100);
        let parent_one = Solution::new(vec![0.0; 50]);

        crossover_func.apply(&parent_one, &parent_one.clone());
    }

    #[test]
    fn test_local_exchange() {
        let graph = FatTree::new(4);
        let dm = build_cache(&graph, 5);

        let crossover_func_2 = LocalExchange::new(1.0, 0.2, &dm);
        let crossover_func_3 = LocalExchange::new(1.0, 0.4, &dm);
        let crossover_func_4 = LocalExchange::new(1.0, 0.6, &dm);
        let crossover_func_5 = LocalExchange::new(1.0, 0.8, &dm);
        let crossover_func_6 = LocalExchange::new(1.0, 1.0, &dm);

        basic_tests(crossover_func_2, graph.num_servers);
        basic_tests(crossover_func_3, graph.num_servers);
        basic_tests(crossover_func_4, graph.num_servers);
        basic_tests(crossover_func_5, graph.num_servers);
        basic_tests(crossover_func_6, graph.num_servers);
    }

    #[test]
    #[should_panic(expected = "The distance matrix is empty.")]
    fn test_local_exchange_empty() {
        let prop_exchange = 0.5;

        // Empty DM
        LocalExchange::new(1.0, prop_exchange, &vec![]);
    }

    fn basic_tests(crossover_func: impl Crossover<Option<usize>>, num_servers: usize) {
        let empty_parent = Solution::new(vec![None; num_servers]);
        let mut parent_one = empty_parent.clone();

        for i in 0..parent_one.len() {
            parent_one[i] = Some(i);
        }

        // Check that the right number of instances of each number exists
        // and that they were distributed between the two children
        let mut used_child_one = false;
        let mut used_child_two = false;

        for _ in 0..10 {
            let children = crossover_func.apply(&parent_one, &empty_parent);

            let child_one = &children[0];
            let child_two = &children[1];

            assert_eq!(child_one.len(), num_servers);
            assert_eq!(child_two.len(), num_servers);

            let mut numbers = vec![0; num_servers];
            for i in 0..num_servers {
                if let Some(x) = child_one[i] {
                    numbers[x] = numbers[x] + 1;
                    used_child_one = true;
                }

                if let Some(x) = child_two[i] {
                    numbers[x] = numbers[x] + 1;
                    used_child_two = true;
                }
            }

            for i in 0..num_servers {
                assert_eq!(numbers[i], 1);
            }
        }

        assert!(used_child_one);
        assert!(used_child_two);
    }
}
