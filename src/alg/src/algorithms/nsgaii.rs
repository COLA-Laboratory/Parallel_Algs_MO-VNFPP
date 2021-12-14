use crate::operators::{
    crossover::Crossover, evaluation::Evaluation, initialisation::InitPop, mapping::Mapping,
    mutation::Mutation, selection::TournamentSelection, solution::Constraint, solution::Solution,
};
use std::cmp::Ordering;
use std::fmt::Debug;

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

    parent_pop.iter_mut().for_each(|ind| {
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
            .into_iter()
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

pub fn fast_nondominated_sort<X: Clone>(
    pop: &mut Vec<NSGAII_Solution<X>>,
) -> Vec<Vec<NSGAII_Solution<X>>> {
    let mut dominates = Vec::with_capacity(pop.len());
    let mut dom_counted = Vec::with_capacity(pop.len());

    let mut ranks = vec![Vec::new()];
    let mut output = vec![Vec::new()];

    for p in 0..pop.len() {
        let mut p_dominates = Vec::new();
        let mut dom_count = 0;

        for q in 0..pop.len() {
            if pop[p].dominates(&pop[q]) {
                p_dominates.push(q);
            } else if pop[q].dominates(&pop[p]) {
                dom_count = dom_count + 1;
            }
        }

        if dom_count == 0 {
            pop[p].rank = 0;
            ranks[0].push(p);

            let ind = pop[p].clone();
            output[0].push(ind);
        }

        dominates.push(p_dominates);
        dom_counted.push(dom_count);
    }

    let mut i = 0;
    while !ranks[i].is_empty() {
        let mut next_rank = Vec::new();
        let mut next_output = Vec::new();

        for p in &ranks[i] {
            for q in &dominates[*p] {
                dom_counted[*q] -= 1;

                if dom_counted[*q] == 0 {
                    pop[*q].rank = i + 1;
                    next_rank.push(*q);

                    next_output.push(pop[*q].clone());
                }
            }
        }

        i = i + 1;
        ranks.push(next_rank);
        output.push(next_output);
    }

    output
}

pub fn crowding_distance_assignment<X: Clone + Debug>(pop: &mut [NSGAII_Solution<X>]) {
    // If population is empty or there are no feasible solutions
    if !pop.iter().any(|ind| ind.objectives().is_feasible()) {
        return;
    }

    let num_obj = pop[0].objectives().unwrap().len();

    for ind in pop.iter_mut() {
        ind.crowding_dist = 0.0;
    }

    let mut idxs: Vec<usize> = (0..pop.len()).into_iter().collect();

    for m in 0..num_obj {
        idxs.sort_by(|&x, &y| {
            pop[x].objectives().unwrap()[m]
                .partial_cmp(&pop[y].objectives().unwrap()[m])
                .unwrap()
        });

        let l = pop.len() - 1;

        let min_idx = idxs[0];
        let max_idx = idxs[l];

        pop[min_idx].crowding_dist = std::f64::INFINITY;
        pop[max_idx].crowding_dist = std::f64::INFINITY;

        let obj_min = pop[min_idx].objectives().unwrap()[m];
        let obj_max = pop[max_idx].objectives().unwrap()[m];

        let diff = if obj_min == obj_max {
            1.0
        } else {
            obj_max - obj_min
        };

        if l <= 1 {
            continue;
        }

        for i in 1..l {
            let curr = idxs[i];
            let next = idxs[i + 1];
            let pre = idxs[i - 1];

            pop[curr].crowding_dist +=
                (pop[next].objectives().unwrap()[m] - pop[pre].objectives().unwrap()[m]) / diff;
        }
    }
}

pub fn crowding_comparison_operator<X: Clone>(
    ind_a: &NSGAII_Solution<X>,
    ind_b: &NSGAII_Solution<X>,
) -> Ordering {
    if ind_a.rank < ind_b.rank
        || (ind_a.rank == ind_b.rank && ind_a.crowding_dist > ind_b.crowding_dist)
    {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

// Wrapper around Solution struct with extra information for NSGA-II
#[derive(Clone, Debug)]
#[allow(non_camel_case_types)]
pub struct NSGAII_Solution<X: Clone> {
    pub solution: Solution<X>,
    pub crowding_dist: f64,
    pub rank: usize,
}

impl<X: Clone> NSGAII_Solution<X> {
    pub fn new(solution: Solution<X>) -> NSGAII_Solution<X> {
        NSGAII_Solution {
            solution,
            crowding_dist: 0.0,
            rank: 0,
        }
    }

    pub fn dominates(&self, other: &NSGAII_Solution<X>) -> bool {
        self.solution.dominates(&other.solution)
    }

    pub fn objectives(&self) -> &Constraint<Vec<f64>, usize> {
        &self.solution.objectives
    }
}
