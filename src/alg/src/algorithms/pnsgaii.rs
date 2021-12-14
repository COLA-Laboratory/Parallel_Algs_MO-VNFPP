use rand::{prelude::SliceRandom, prelude::ThreadRng, thread_rng};
use rayon::iter::{IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator};

use crate::operators::{
    crossover::Crossover, evaluation::Evaluation, initialisation::InitPop, mapping::Mapping,
    mutation::Mutation, selection::TournamentSelection, solution::Solution,
};
use std::cmp::Ordering;
use std::fmt::Debug;

use super::nsgaii::{
    crowding_comparison_operator, crowding_distance_assignment, fast_nondominated_sort,
    NSGAII_Solution,
};

// Comparison of Parallel Genetic Algorithm and Particle Swarm Optimization for Real-Time UAV Path Planning
pub fn run<
    X: Sync,
    Init: InitPop<X>,
    Map: Mapping<X> + Sync,
    Eval: Evaluation + Sync,
    Mutate: Mutation<X> + Sync,
    Cross: Crossover<X> + Sync,
>(
    init_pop: &Init,
    mapping: &Map,
    evaluate: &Eval,
    mutation: &Mutate,
    crossover: &Cross,
    pop_size: usize,
    max_evaluations: usize,
    num_epochs: usize,
    mut iteration_observer: impl FnMut(usize, &Vec<Solution<X>>),
) where
    X: Clone + Debug + Send,
{
    let mut global_pop: Vec<NSGAII_Solution<X>> = init_pop
        .apply(pop_size)
        .into_iter()
        .map(|solution| NSGAII_Solution::new(solution))
        .collect();

    global_pop.par_iter_mut().for_each(|ind| {
        let routes = mapping.apply(&ind.solution);
        ind.solution.objectives = evaluate.evaluate_ind(&routes)
    });

    let num_cores = num_cpus::get();
    let max_sub_evaluations = (max_evaluations - global_pop.len()) / (num_cores * num_epochs);
    let sub_pop_size = pop_size / num_cores;

    let mut rng = thread_rng();

    for _ in 0..num_epochs {
        let pops = scatter_pop(&mut global_pop, num_cores, sub_pop_size, &mut rng);

        global_pop = pops
            .into_par_iter()
            .flat_map(|mut pop| {
                let mut num_sub_evaluations = 0;

                // Initial population
                let mut child_pop = Vec::with_capacity(sub_pop_size);
                let mut combined_pop = Vec::with_capacity(pop_size * 2);

                while num_sub_evaluations < max_sub_evaluations {
                    combined_pop.clear();
                    combined_pop.append(&mut pop);
                    combined_pop.append(&mut child_pop);

                    let mut nondominated_fronts = fast_nondominated_sort(&mut combined_pop);

                    let mut i = 0;
                    while pop.len() + nondominated_fronts[i].len() < sub_pop_size {
                        crowding_distance_assignment(&mut nondominated_fronts[i]);

                        pop.append(&mut nondominated_fronts[i]);
                        i = i + 1;
                    }

                    nondominated_fronts[i].sort_by(|x, y| crowding_comparison_operator(x, y));

                    for j in 0..(sub_pop_size - pop.len()) {
                        pop.push(nondominated_fronts[i][j].clone());
                    }

                    let ts = TournamentSelection::new(pop.len(), |x, y| {
                        crowding_comparison_operator(&pop[x], &pop[y]) == Ordering::Less
                    });

                    child_pop = (0..(sub_pop_size / 2))
                        .into_iter()
                        .flat_map(|_| {
                            let parent_one = ts.tournament(2);
                            let parent_two = ts.tournament(2);

                            let new_children = crossover
                                .apply(&pop[parent_one].solution, &pop[parent_two].solution);

                            let children = Vec::with_capacity(2);

                            for child in new_children {
                                let solution = mutation.apply(&child);
                                let mut new_child = NSGAII_Solution::new(solution);

                                let routes = mapping.apply(&new_child.solution);
                                new_child.solution.objectives = evaluate.evaluate_ind(&routes);
                            }

                            children
                        })
                        .collect();

                    num_sub_evaluations = num_sub_evaluations + sub_pop_size;
                }

                combined_pop
            })
            .collect();
    }

    iteration_observer(
        max_sub_evaluations * num_cores,
        &global_pop.iter().map(|ind| ind.solution.clone()).collect(),
    );
}

fn scatter_pop<X: Clone>(
    global_pop: &mut Vec<NSGAII_Solution<X>>,
    num_cores: usize,
    sub_pop_size: usize,
    rng: &mut ThreadRng,
) -> Vec<Vec<NSGAII_Solution<X>>> {
    // Split global pop into sub populations
    global_pop.shuffle(rng);

    let mut new_pops = vec![Vec::new(); num_cores];

    let mut pos = 0;
    for i in 0..num_cores {
        for _ in 0..sub_pop_size {
            new_pops[i].push(global_pop[pos].clone());

            pos = pos + 1;

            if pos == global_pop.len() {
                global_pop.shuffle(rng);
                pos = 0;
            }
        }
    }

    new_pops
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_scatter_pop() {
        let sols: Vec<Solution<_>> = (0..8).into_iter().map(|x| Solution::new(vec![x])).collect();

        let mut global_pop: Vec<NSGAII_Solution<_>> = sols
            .into_iter()
            .map(|sol| NSGAII_Solution::new(sol))
            .collect();

        let mut rng = thread_rng();

        let pops_two = scatter_pop(&mut global_pop, 2, 4, &mut rng);
        let pops_four = scatter_pop(&mut global_pop, 4, 2, &mut rng);
        let pops_double = scatter_pop(&mut global_pop, 2, 8, &mut rng);

        // Two pops
        assert_eq!(pops_two.len(), 2);
        assert_eq!(pops_two[0].len(), 4);
        assert_eq!(pops_two[1].len(), 4);

        let mut sum = 0;
        for i in 0..4 {
            sum = sum + pops_two[0][i].solution[0] + pops_two[1][i].solution[0];
        }
        assert_eq!(sum, 28);

        // Four pops
        assert_eq!(pops_four.len(), 4);
        assert_eq!(pops_four[0].len(), 2);
        assert_eq!(pops_four[1].len(), 2);
        assert_eq!(pops_four[2].len(), 2);
        assert_eq!(pops_four[3].len(), 2);

        let mut sum = 0;
        for i in 0..2 {
            sum = sum
                + pops_four[0][i].solution[0]
                + pops_four[1][i].solution[0]
                + pops_four[2][i].solution[0]
                + pops_four[3][i].solution[0];
        }
        assert_eq!(sum, 28);

        // Duplicated pops
        assert_eq!(pops_double.len(), 2);
        assert_eq!(pops_double[0].len(), 8);
        assert_eq!(pops_double[1].len(), 8);

        let mut sum = 0;
        for i in 0..8 {
            sum = sum + pops_double[0][i].solution[0] + pops_double[1][i].solution[0];
        }
        assert_eq!(sum, 56);
    }
}
