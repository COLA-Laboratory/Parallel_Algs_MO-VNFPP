use crate::operators::solution::Solution;

pub struct NonDominatedSet<X> {
    accept_duplicates: bool,
    archive: Vec<Solution<X>>,
}

impl<X> NonDominatedSet<X> {
    pub fn new(accept_duplicates: bool) -> NonDominatedSet<X> {
        NonDominatedSet {
            accept_duplicates,
            archive: Vec::new(),
        }
    }

    pub fn try_push(&mut self, solution: Solution<X>) -> bool {
        let mut is_dominated = false;
        let mut is_duplicate = false;

        for i in (0..self.archive.len()).rev() {
            let curr_ind = &self.archive[i];

            if curr_ind.dominates(&solution) {
                is_dominated = true;
            } else if solution.dominates(curr_ind) {
                self.archive.swap_remove(i);
            } else if !self.accept_duplicates && curr_ind.objectives == solution.objectives {
                is_duplicate = true;
            }

            if is_dominated {
                break;
            }
        }

        if !is_dominated && !is_duplicate {
            self.archive.push(solution);
        }

        !is_dominated
    }

    pub fn try_push_with(
        &mut self,
        solution: Solution<X>,
        dominates: impl Fn(&Solution<X>, &Solution<X>) -> bool,
    ) -> bool {
        let mut is_dominated = false;
        let mut is_duplicate = false;

        for i in (0..self.archive.len()).rev() {
            let curr_ind = &self.archive[i];

            if dominates(&curr_ind, &solution) {
                is_dominated = true;
            } else if dominates(&solution, &curr_ind) {
                self.archive.swap_remove(i);
            } else if !self.accept_duplicates && curr_ind.objectives == solution.objectives {
                is_duplicate = true;
            }

            if is_dominated {
                break;
            }
        }

        if !is_dominated && !is_duplicate {
            self.archive.push(solution);
        }

        !is_dominated
    }

    pub fn get_raw(&self) -> &Vec<Solution<X>> {
        &self.archive
    }
}

#[cfg(test)]
mod tests {
    use crate::operators::solution::Constraint;

    use super::*;

    #[test]
    pub fn test_try_push() {
        let mut solutions = vec![Solution::new(vec![0.0]); 5];

        // All feasible, accept duplicates
        let mut f_set = NonDominatedSet::new(true);

        solutions[0].objectives = Constraint::Feasible(vec![5.0, 5.0]);
        solutions[1].objectives = Constraint::Feasible(vec![3.0, 4.0]);
        solutions[2].objectives = Constraint::Feasible(vec![4.0, 6.0]);
        solutions[3].objectives = Constraint::Feasible(vec![3.0, 4.0]);
        solutions[4].objectives = Constraint::Feasible(vec![4.0, 2.0]);

        let f_a = f_set.try_push(solutions[0].clone());
        let f_b = f_set.try_push(solutions[1].clone());
        let f_c = f_set.try_push(solutions[2].clone());
        let f_d = f_set.try_push(solutions[3].clone());
        let f_e = f_set.try_push(solutions[4].clone());

        // All infeasible
        let mut i_set = NonDominatedSet::new(true);

        solutions[0].objectives = Constraint::Infeasible(3);
        solutions[1].objectives = Constraint::Infeasible(4);
        solutions[2].objectives = Constraint::Infeasible(2);
        solutions[3].objectives = Constraint::Infeasible(1);
        solutions[4].objectives = Constraint::Infeasible(5);

        let i_a = i_set.try_push(solutions[0].clone());
        let i_b = i_set.try_push(solutions[1].clone());
        let i_c = i_set.try_push(solutions[2].clone());
        let i_d = i_set.try_push(solutions[3].clone());
        let i_e = i_set.try_push(solutions[4].clone());

        // Mixed
        let mut m_set = NonDominatedSet::new(true);

        solutions[0].objectives = Constraint::Infeasible(3);
        solutions[1].objectives = Constraint::Infeasible(2);
        solutions[2].objectives = Constraint::Feasible(vec![3.0, 4.0]);
        solutions[3].objectives = Constraint::Infeasible(1);
        solutions[4].objectives = Constraint::Feasible(vec![4.0, 3.0]);

        let m_a = m_set.try_push(solutions[0].clone());
        let m_b = m_set.try_push(solutions[1].clone());
        let m_c = m_set.try_push(solutions[2].clone());
        let m_d = m_set.try_push(solutions[3].clone());
        let m_e = m_set.try_push(solutions[4].clone());

        // Refuse duplicates
        let mut d_set = NonDominatedSet::new(false);

        solutions[0].objectives = Constraint::Feasible(vec![3.0, 4.0]);
        solutions[1].objectives = Constraint::Feasible(vec![4.0, 5.0]);
        solutions[2].objectives = Constraint::Feasible(vec![3.0, 4.0]);
        solutions[3].objectives = Constraint::Feasible(vec![3.1, 3.9]);
        solutions[4].objectives = Constraint::Feasible(vec![4.0, 3.0]);

        let d_a = d_set.try_push(solutions[0].clone());
        let d_b = d_set.try_push(solutions[1].clone());
        let d_c = d_set.try_push(solutions[2].clone());
        let d_d = d_set.try_push(solutions[3].clone());
        let d_e = d_set.try_push(solutions[4].clone());

        assert!(f_a);
        assert!(f_b);
        assert!(!f_c);
        assert!(f_d);
        assert!(f_e);

        assert_eq!(f_set.get_raw().len(), 3);

        assert!(i_a);
        assert!(!i_b);
        assert!(i_c);
        assert!(i_d);
        assert!(!i_e);

        assert_eq!(i_set.get_raw().len(), 1);

        assert!(m_a);
        assert!(m_b);
        assert!(m_c);
        assert!(!m_d);
        assert!(m_e);

        assert_eq!(m_set.get_raw().len(), 2);

        assert!(d_a);
        assert!(!d_b);
        assert!(!d_c);
        assert!(d_d);
        assert!(d_e);

        assert_eq!(d_set.get_raw().len(), 3);
    }
}
