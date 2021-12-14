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
    mut iteration_observer: impl FnMut(usize, &Vec<Solution<X>>),
) where
    X: Clone + Debug + Send,
{
    let mut parent_pop: Vec<NSGAII_Solution<X>> = init_pop
        .apply(pop_size)
        .into_iter()
        .map(|solution| NSGAII_Solution::new(solution))
        .collect();

    parent_pop.par_iter_mut().for_each(|ind| {
        let routes = mapping.apply(&ind.solution);
        ind.solution.objectives = evaluate.evaluate_ind(&routes)
    });

    let mut evaluations = parent_pop.len();

    // Initial population
    let mut child_pop = Vec::with_capacity(pop_size);
    let mut combined_pop = Vec::with_capacity(pop_size * 2);

    while evaluations < max_evaluations {
        combined_pop.clear();
        combined_pop.append(&mut parent_pop);
        combined_pop.append(&mut child_pop);

        let mut nondominated_fronts = fast_nondominated_sort(&mut combined_pop);

        let mut i = 0;
        while parent_pop.len() + nondominated_fronts[i].len() < pop_size {
            crowding_distance_assignment(&mut nondominated_fronts[i]);

            parent_pop.append(&mut nondominated_fronts[i]);
            i = i + 1;
        }

        nondominated_fronts[i].sort_by(|x, y| crowding_comparison_operator(x, y));

        for j in 0..(pop_size - parent_pop.len()) {
            parent_pop.push(nondominated_fronts[i][j].clone());
        }

        let ts = TournamentSelection::new(parent_pop.len(), |x, y| {
            crowding_comparison_operator(&parent_pop[x], &parent_pop[y]) == Ordering::Less
        });

        child_pop = (0..(pop_size / 2))
            .into_par_iter()
            .flat_map(|_| {
                let parent_one = ts.tournament(2);
                let parent_two = ts.tournament(2);

                let new_children = crossover.apply(
                    &parent_pop[parent_one].solution,
                    &parent_pop[parent_two].solution,
                );

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

        evaluations = evaluations + pop_size;
    }

    iteration_observer(
        evaluations,
        &parent_pop.iter().map(|ind| ind.solution.clone()).collect(),
    );
}
