use rand::prelude::*;

use crate::{models::service::Service, operators::solution::Solution};

pub trait InitPop<X> {
    fn apply(&self, pop_size: usize) -> Vec<Solution<X>>;
}

pub struct SAIntStringInitialisation<'a> {
    services: &'a Vec<Service>,
    solution_length: usize,
}

impl SAIntStringInitialisation<'_> {
    pub fn new(services: &Vec<Service>, solution_length: usize) -> SAIntStringInitialisation {
        SAIntStringInitialisation {
            services,
            solution_length,
        }
    }
}

impl<'a> InitPop<usize> for SAIntStringInitialisation<'a> {
    fn apply(&self, pop_size: usize) -> Vec<Solution<usize>> {
        let mut population = Vec::new();

        let mut min_size = 0;
        for service in self.services {
            for vnf in &service.vnfs {
                min_size = min_size + vnf.size;
            }
        }

        let max_size = self.solution_length * 100;

        for i in 0..pop_size {
            let prop = i as f64 / pop_size as f64;

            // Add all service instances
            let used_size = prop * max_size as f64;

            let num_instances = (used_size / min_size as f64) as usize;
            let num_instances = num_instances.max(1); // At least one instance

            let new_solution = vec![num_instances; self.services.len()];
            population.push(Solution::new(new_solution));
        }

        population
    }
}

pub struct ServiceAwareInitialisation<'a> {
    services: &'a Vec<Service>,
    solution_length: usize,
}

impl ServiceAwareInitialisation<'_> {
    pub fn new(services: &Vec<Service>, solution_length: usize) -> ServiceAwareInitialisation {
        ServiceAwareInitialisation {
            services,
            solution_length,
        }
    }
}

impl<'a> InitPop<Vec<&'a Service>> for ServiceAwareInitialisation<'a> {
    fn apply(&self, pop_size: usize) -> Vec<Solution<Vec<&'a Service>>> {
        let mut population = Vec::new();
        let mut rng = rand::thread_rng();

        let mut min_size = 0;
        for service in self.services {
            for vnf in &service.vnfs {
                min_size = min_size + vnf.size;
            }
        }

        let max_size = self.solution_length * 100;

        for i in 0..pop_size {
            let mut new_solution = vec![Vec::new(); self.solution_length];
            let prop = i as f64 / pop_size as f64;

            // Add all service instances
            let mut num_placed = 0;

            let used_size = prop * (min_size + (max_size - min_size)) as f64;

            let num_instances = (used_size / min_size as f64) as usize;
            let num_instances = num_instances.max(1); // At least one instance

            for service in self.services {
                for _ in 0..num_instances {
                    let pos = rng.gen_range(0, self.solution_length);
                    new_solution[pos].push(service);

                    num_placed = num_placed + 1;
                }
            }

            population.push(Solution::new(new_solution));
        }

        population
    }
}

// TODO: Write tests for service aware distribution

// #[cfg(test)]
// mod tests {
//     use crate::operators::initialisation::*;

//     #[test]
//     fn test_initialisation() {
//         let num_characters = 5;
//         let characters: Vec<usize> = (0..num_characters).into_iter().collect();
//         let weights: Vec<f64> = characters.iter().map(|i| *i as f64).collect();

//         let distribution = WeightedIndex::new(weights).unwrap();

//         let length = 1000;
//         let num_ind = 100;

//         let init =
//             UniformInitialisation::new_weighted_distribution(&characters, length, distribution);
//         let population = init.apply(num_ind);

//         assert_eq!(population.len(), num_ind);

//         let mut distr = vec![0; characters.len()];
//         for ind in population {
//             assert_eq!(ind.len(), length);

//             // Check only the characters provided are being used
//             for s in ind.point {
//                 for c in s {
//                     assert!(characters.contains(&c));

//                     let pos = characters.iter().position(|&e| e == c).unwrap();
//                     distr[pos] = distr[pos] + 1;
//                 }
//             }
//         }

//         // Check that characters are distributed correctly
//         let total_weight = (0..num_characters).into_iter().sum::<usize>();

//         for i in 0..characters.len() {
//             let measured_dist = distr[i] as f64 / (length * num_ind) as f64;
//             let expected_distr = i as f64 / total_weight as f64;

//             assert!(measured_dist < expected_distr + 0.05 && measured_dist > expected_distr - 0.05);
//         }
//     }
// }
