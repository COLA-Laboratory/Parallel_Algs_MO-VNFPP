use std::fmt::Debug;

use rand::Rng;
use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};

use crate::{
    operators::mapping::Mapping,
    operators::neighbour_gen::NeighbourGenerator,
    operators::{
        evaluation::Evaluation, initialisation::InitPop, solution::Constraint, solution::Solution,
    },
    utilities::nds::NonDominatedSet,
};

pub fn run<
    X,
    Init: InitPop<X>,
    Map: Mapping<X> + Sync,
    NeighbourGen: NeighbourGenerator<X> + Sync,
    Eval: Evaluation + Sync + Clone,
>(
    init_pop: &Init,
    mapping: &Map,
    evaluate: &Eval,
    neighbour_gen: &NeighbourGen,
    pop_size: usize,
    max_evaluations: usize,
    per_ind_evaluations: usize,
    num_obj: usize,
    iteration_observer: impl Fn(usize, &Vec<Solution<X>>) + Sync,
) where
    X: Clone + Debug + Sync + Send,
{
    // Evaluate initial pop
    let weight_vectors = get_weights(pop_size, num_obj);
    let mut init_archive = init_pop.apply(pop_size);

    init_archive.par_iter_mut().for_each(|ind| {
        let routes = mapping.apply(&ind);
        ind.objectives = evaluate.evaluate_ind(&routes)
    });

    let (ref_point, nadir_point) = get_ref_points(&init_archive, num_obj);

    let remaining_evaluations = max_evaluations - pop_size;
    let per_weight_evaluations = remaining_evaluations / pop_size;

    let total_archive: Vec<NonDominatedSet<X>> = weight_vectors
        .par_iter()
        .map(|wv| {
            let evaluate = evaluate.clone();

            // Pick the best starting individual for the current weight
            let (best_idx, _, _) = get_best(&init_archive, &wv, &ref_point, &nadir_point);
            let best_ind = &init_archive[best_idx];

            // Create archives
            let mut archive = NonDominatedSet::new(false);
            archive.try_push_with(best_ind.clone(), |x, y| {
                dominates(x, y, wv, &weight_vectors)
            });

            let mut evaluations = 0;
            while evaluations < per_weight_evaluations {
                // Choose a random solution from the archive
                let mut rng = rand::thread_rng();
                let idx = rng.gen_range(0, archive.get_raw().len());

                let best_ind = archive.get_raw()[idx].clone();

                // Generate and evalute neighbouring solutions
                for _ in 0..per_ind_evaluations {
                    let mut ind = neighbour_gen.apply(&best_ind);

                    let routes = mapping.apply(&ind);
                    ind.objectives = evaluate.evaluate_ind(&routes);

                    archive.try_push_with(ind, |x, y| dominates(x, y, wv, &weight_vectors));
                }

                evaluations = evaluations + per_ind_evaluations;
            }

            archive
        })
        .collect();

    // Merge non-dominated sets and report result
    let mut final_solutions = NonDominatedSet::new(false);
    for set in total_archive {
        for solution in set.get_raw() {
            final_solutions.try_push(solution.clone());
        }
    }

    iteration_observer(max_evaluations, final_solutions.get_raw());
}

fn dominates<X>(
    x: &Solution<X>,
    y: &Solution<X>,
    weight: &Vec<f64>,
    other_weights: &Vec<Vec<f64>>,
) -> bool {
    dominates_cnstr(&x, &y) || dominates_angle(x, y, weight, other_weights) || x.dominates(y)
}

fn dominates_cnstr<X>(x: &Solution<X>, y: &Solution<X>) -> bool {
    match (&x.objectives, &y.objectives) {
        (Constraint::Feasible(_), Constraint::Feasible(_)) => false,
        // Infeasible solutions are dominated by any feasible one
        (Constraint::Feasible(_), Constraint::Infeasible(_)) => true,
        (Constraint::Infeasible(_), Constraint::Feasible(_)) => false,
        // Infeasible solutions with a lower rating are better
        (Constraint::Infeasible(x), Constraint::Infeasible(y)) => x < y,
        _ => panic!("Undefined objectives"),
    }
}

fn dominates_angle<X>(
    x: &Solution<X>,
    y: &Solution<X>,
    ref_weight: &Vec<f64>,
    other_weights: &Vec<Vec<f64>>,
) -> bool {
    let x_obj = &x.objectives.unwrap();
    let y_obj = &y.objectives.unwrap();

    let in_region_x = is_in_region(x_obj, ref_weight, other_weights);
    let in_region_y = is_in_region(y_obj, ref_weight, other_weights);

    let ang_a = angle(x_obj, ref_weight);
    let ang_b = angle(y_obj, ref_weight);

    match (in_region_x, in_region_y) {
        (true, true) => false,
        (false, true) => false,
        (true, false) => true,
        (false, false) => ang_a < ang_b,
    }
}

fn get_best<'a, X: Clone>(
    pop: &'a Vec<Solution<X>>,
    wv: &Vec<f64>,
    ref_point: &Vec<f64>,
    nadir_point: &Vec<f64>,
) -> (usize, f64, usize) {
    let mut best_ind = 0;
    let mut min_dist = std::f64::INFINITY;
    let mut min_infeasible = std::usize::MAX;

    for (i, ind) in pop.iter().enumerate() {
        match (&ind.objectives, &pop[best_ind].objectives) {
            (Constraint::Feasible(ind_objectives), _) => {
                let dist = tchebycheff(&ind_objectives, &wv, &ref_point, &nadir_point);

                if dist < min_dist {
                    min_dist = dist;
                    best_ind = i;
                    min_infeasible = 0;
                }
            }
            (Constraint::Infeasible(ind_constraint), Constraint::Infeasible(_)) => {
                if *ind_constraint < min_infeasible {
                    min_infeasible = *ind_constraint;
                    best_ind = i;
                }
            }
            (Constraint::Infeasible(_), Constraint::Feasible(_)) => {
                // do nothing
            }
            _ => panic!("One or more objectives undefined"),
        }
    }

    (best_ind, min_dist, min_infeasible)
}

fn is_in_region(solution: &Vec<f64>, weight: &Vec<f64>, other_weights: &Vec<Vec<f64>>) -> bool {
    let cmp_angle = angle(solution, weight);

    for oth_weight in other_weights {
        let oth_angle = angle(solution, oth_weight);

        if oth_angle < cmp_angle {
            return false;
        }
    }

    true
}

fn angle(vec_a: &Vec<f64>, vec_b: &Vec<f64>) -> f64 {
    (dot_product(vec_a, vec_b) / (magnitude(vec_a) * magnitude(vec_b))).acos()
}

fn magnitude(vec_a: &Vec<f64>) -> f64 {
    vec_a.iter().map(|c| c.powf(2.0)).sum::<f64>().sqrt()
}

fn dot_product(vec_a: &Vec<f64>, vec_b: &Vec<f64>) -> f64 {
    vec_a.iter().zip(vec_b).map(|(a, b)| a * b).sum()
}

fn get_weights(pop_size: usize, num_obj: usize) -> Vec<Vec<f64>> {
    let pop_size = pop_size as i32;

    let pop_to_h = vec![
        28, 36, 45, 55, 66, 78, 91, 105, 120, 136, 153, 171, 190, 210, 231, 253, 276, 300, 325,
        351, 378, 406, 435, 465, 496, 528, 561, 595,
    ];

    let mut dist = pop_size - pop_to_h[0];

    let mut i = 0;

    loop {
        let c_dist = (pop_size - pop_to_h[i]).abs();
        if c_dist < dist {
            dist = c_dist;
        }

        if c_dist > dist {
            break;
        }

        i = i + 1;
    }

    let h = i + 5;

    let mut weights = Vec::new();
    for i in 0..=h {
        for j in 0..=h {
            if i + j <= h {
                let k = h - i - j;
                let mut weight = Vec::with_capacity(num_obj);

                weight.push(i as f64 / h as f64);
                weight.push(j as f64 / h as f64);
                weight.push(k as f64 / h as f64);

                // Normalise weight
                let mag = weight.iter().map(|w| w.powf(2.0)).sum::<f64>().sqrt();
                let weight = weight.into_iter().map(|w| w / mag).collect();

                weights.push(weight);
            }
        }
    }

    weights
}

fn tchebycheff(
    objectives: &Vec<f64>,
    weights: &Vec<f64>,
    ref_point: &Vec<f64>,
    nadir_point: &Vec<f64>,
) -> f64 {
    let mut max = std::f64::MIN;

    for i in 0..objectives.len() {
        let dist =
            weights[i] * ((objectives[i] - ref_point[i]) / (nadir_point[i] - ref_point[i])).abs();

        if dist > max {
            max = dist;
        }
    }
    max
}

pub fn get_ref_points<X>(population: &Vec<Solution<X>>, num_obj: usize) -> (Vec<f64>, Vec<f64>) {
    let mut ref_point = vec![std::f64::MAX; num_obj];
    let mut nadir_point = vec![std::f64::MIN; num_obj];

    for ind in population {
        if !ind.objectives.is_feasible() {
            continue;
        }

        let obj = ind.objectives.unwrap();

        for i in 0..num_obj {
            if obj[i] < ref_point[i] {
                ref_point[i] = obj[i];
            }

            if obj[i] > nadir_point[i] {
                nadir_point[i] = obj[i];
            }
        }
    }

    (ref_point, nadir_point)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        operators::solution::{Constraint, Solution},
        utilities::math::round_to,
    };

    #[test]
    pub fn test_dominates_angle() {
        let ref_weight = vec![1.0, 1.0];
        let oth_weights = vec![vec![0.0, 1.0], vec![1.0, 0.0]];

        // Yes
        let a = vec![1.0, 1.0];
        let b = vec![0.5, 0.5];

        // No
        let c = vec![1.0, 0.0];
        let d = vec![1.0, 0.2];
        let e = vec![1.0, 0.3];

        let mut x = Solution::new(vec![0]);
        let mut y = Solution::new(vec![0]);

        // Both in range
        x.objectives = Constraint::Feasible(a.clone());
        y.objectives = Constraint::Feasible(b.clone());

        let bir_a = dominates_angle(&x, &y, &ref_weight, &oth_weights);
        let bir_b = dominates_angle(&y, &x, &ref_weight, &oth_weights);

        // One in range
        x.objectives = Constraint::Feasible(a.clone());
        y.objectives = Constraint::Feasible(c.clone());

        let oir_a = dominates_angle(&x, &y, &ref_weight, &oth_weights);
        let oir_b = dominates_angle(&y, &x, &ref_weight, &oth_weights);

        // Both out of range
        x.objectives = Constraint::Feasible(d.clone());
        y.objectives = Constraint::Feasible(e.clone());

        let bor_a = dominates_angle(&x, &y, &ref_weight, &oth_weights);
        let bor_b = dominates_angle(&y, &x, &ref_weight, &oth_weights);

        assert!(!bir_a);
        assert!(!bir_b);

        assert!(oir_a);
        assert!(!oir_b);

        assert!(!bor_a);
        assert!(bor_b);
    }

    #[test]
    pub fn test_get_best() {
        let mut solutions = vec![Solution::new(vec![0]); 4];

        let wv = vec![1.0, 1.0];
        let ref_point = vec![0.0, 0.0];
        let nadir_point = vec![1.0, 1.0];

        // All feasible
        solutions[0].objectives = Constraint::Feasible(vec![0.0, 1.0]);
        solutions[1].objectives = Constraint::Feasible(vec![0.25, 0.75]);
        solutions[2].objectives = Constraint::Feasible(vec![0.5, 0.5]);
        solutions[3].objectives = Constraint::Feasible(vec![0.75, 0.25]);

        let (af_id, af_dist, af_cv) = get_best(&solutions, &wv, &ref_point, &nadir_point);

        // Mixed
        solutions[0].objectives = Constraint::Infeasible(3);
        solutions[1].objectives = Constraint::Feasible(vec![0.45, 0.55]);
        solutions[2].objectives = Constraint::Infeasible(1);
        solutions[3].objectives = Constraint::Feasible(vec![0.75, 0.25]);

        let (m_id, m_dist, m_cv) = get_best(&solutions, &wv, &ref_point, &nadir_point);

        // All infeasible
        solutions[0].objectives = Constraint::Infeasible(3);
        solutions[1].objectives = Constraint::Infeasible(4);
        solutions[2].objectives = Constraint::Infeasible(5);
        solutions[3].objectives = Constraint::Infeasible(4);

        let (if_id, if_dist, if_cv) = get_best(&solutions, &wv, &ref_point, &nadir_point);

        assert_eq!(af_id, 2);
        assert_eq!(af_dist, 0.5);
        assert_eq!(af_cv, 0);

        assert_eq!(m_id, 1);
        assert_eq!(m_dist, 0.55);
        assert_eq!(m_cv, 0);

        assert_eq!(if_id, 0);
        assert_eq!(if_dist, std::f64::INFINITY);
        assert_eq!(if_cv, 3);
    }

    #[test]
    pub fn test_in_region() {
        let ref_weight = vec![1.0, 1.0];
        let oth_weights = vec![vec![0.0, 1.0], vec![1.0, 0.0]];

        // Yes
        let a = vec![1.0, 1.0];
        let b = vec![0.5, 0.5];

        // No
        let c = vec![0.0, 1.0];
        let d = vec![0.2, 1.0];
        let e = vec![3.0, 1.0];

        let t_a = is_in_region(&a, &ref_weight, &oth_weights);
        let t_b = is_in_region(&b, &ref_weight, &oth_weights);
        let t_c = is_in_region(&c, &ref_weight, &oth_weights);
        let t_d = is_in_region(&d, &ref_weight, &oth_weights);
        let t_e = is_in_region(&e, &ref_weight, &oth_weights);

        assert!(t_a);
        assert!(t_b);
        assert!(!t_c);
        assert!(!t_d);
        assert!(!t_e);
    }

    #[test]
    pub fn test_angle() {
        let r = vec![1.0, 1.0];

        let a = vec![2.0, 1.0];
        let b = vec![2.0, 2.0];
        let c = vec![1.0, -1.0];
        let d = vec![0.0, 1.0];

        let ang_a = angle(&r, &a);
        let ang_b = angle(&r, &b);
        let ang_c = angle(&r, &c);
        let ang_d = angle(&r, &d);

        assert_eq!(round_to(ang_a, 3), 0.322);
        assert_eq!(round_to(ang_b, 3), 0.0);
        assert_eq!(round_to(ang_c, 3), 1.571);
        assert_eq!(round_to(ang_d, 3), 0.785);
    }

    #[test]
    pub fn test_magnitude() {
        let x = vec![1.0, 2.0, 3.0];

        let m = magnitude(&x);

        let t: f64 = 14.0;
        assert_eq!(m, t.sqrt());
    }

    #[test]
    pub fn test_dot_product() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![3.0, 5.0, 2.0];

        let dp = dot_product(&x, &y);

        assert_eq!(dp, 19.0);
    }

    #[test]
    pub fn test_tchebycheff() {
        let weights = vec![1.0, 1.0];
        let ref_point = vec![0.0, 0.0];
        let nadir_point = vec![2.0, 2.0];

        let a = vec![0.5, 0.5];
        let b = vec![0.0, 1.0];
        let c = vec![0.5, 2.0];
        let d = vec![2.0, 0.0];
        let e = vec![0.1, 0.1];

        let a_dist = tchebycheff(&a, &weights, &ref_point, &nadir_point);
        let b_dist = tchebycheff(&b, &weights, &ref_point, &nadir_point);
        let c_dist = tchebycheff(&c, &weights, &ref_point, &nadir_point);
        let d_dist = tchebycheff(&d, &weights, &ref_point, &nadir_point);
        let e_dist = tchebycheff(&e, &weights, &ref_point, &nadir_point);

        assert!(a_dist < b_dist && a_dist < c_dist && a_dist < d_dist && a_dist > e_dist);
        assert!(b_dist > a_dist && b_dist < c_dist && b_dist < d_dist && a_dist > e_dist);
        assert!(c_dist > a_dist && c_dist > b_dist && c_dist == d_dist && a_dist > e_dist);
        assert!(d_dist > a_dist && d_dist > b_dist && d_dist == c_dist && a_dist > e_dist);
        assert!(e_dist < a_dist && e_dist < b_dist && e_dist < c_dist && e_dist < d_dist);
    }

    #[test]
    pub fn test_get_ref_point() {
        let mut pop: Vec<Solution<f64>> = (0..5)
            .into_iter()
            .map(|i| Solution::new(vec![i as f64]))
            .collect();

        pop[0].objectives = Constraint::Feasible(vec![2.0, 2.0]);
        pop[1].objectives = Constraint::Feasible(vec![1.8, 2.2]);
        pop[2].objectives = Constraint::Feasible(vec![1.6, 3.1]);
        pop[3].objectives = Constraint::Feasible(vec![3.0, 3.0]);
        pop[4].objectives = Constraint::Feasible(vec![2.1, 1.9]);

        let (utopia, nadir) = get_ref_points(&pop, 2);

        assert_eq!(utopia, vec![1.6, 1.9]);
        assert_eq!(nadir, vec![3.0, 3.1]);
    }
}
