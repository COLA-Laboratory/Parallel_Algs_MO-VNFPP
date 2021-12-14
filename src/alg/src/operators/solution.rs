use std::{
    fmt::{Debug, Display},
    ops::{Index, IndexMut},
};

#[derive(Clone, Debug, PartialEq)]
pub struct Solution<X> {
    pub point: Vec<X>,
    pub objectives: Constraint<Vec<f64>, usize>,
}

impl<X> Solution<X> {
    pub fn new(point: Vec<X>) -> Solution<X> {
        Solution {
            point,
            objectives: Constraint::Undefined,
        }
    }

    pub fn len(&self) -> usize {
        self.point.len()
    }

    pub fn dominates(&self, other: &Solution<X>) -> bool {
        if self.objectives.is_undefined() || other.objectives.is_undefined() {
            panic!("Undefined fitness values");
        }

        match (&self.objectives, &other.objectives) {
            // Infeasible solutions are dominated by any feasible one
            (Constraint::Feasible(_), Constraint::Infeasible(_)) => {
                return true;
            }
            (Constraint::Infeasible(_), Constraint::Feasible(_)) => {
                return false;
            }
            // Infeasible solutions with a lower rating are better
            (Constraint::Infeasible(x), Constraint::Infeasible(y)) => {
                return x < y;
            }
            // Otherwise check dominance
            _ => {}
        }

        // Domination: Equal to or better than in all objective and strictly better than in one
        let self_obj = self.objectives.unwrap();
        let other_obj = other.objectives.unwrap();

        let num_obj = self_obj.len();

        let mut num_better = 0;
        let mut num_worse = 0;

        for i in 0..num_obj {
            if self_obj[i] < other_obj[i] {
                num_better = num_better + 1;
            } else if self_obj[i] > other_obj[i] {
                num_worse = num_worse + 1;
            }
        }

        num_better > 0 && num_worse == 0
    }
}

impl<X> Index<usize> for Solution<X> {
    type Output = X;

    fn index(&self, i: usize) -> &Self::Output {
        &self.point[i]
    }
}

impl<X> IndexMut<usize> for Solution<X> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.point[index]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Constraint<X, Y> {
    Feasible(X),
    Infeasible(Y),
    Undefined,
}

impl<X, Y> Constraint<X, Y>
where
    X: Clone,
    Y: Clone,
{
    pub fn is_feasible(&self) -> bool {
        match self {
            Constraint::Feasible(_) => true,
            _ => false,
        }
    }

    pub fn is_infeasible(&self) -> bool {
        match self {
            Constraint::Infeasible(_) => true,
            _ => false,
        }
    }

    pub fn is_undefined(&self) -> bool {
        match self {
            Constraint::Undefined => true,
            _ => false,
        }
    }

    pub fn unwrap(&self) -> X {
        match self {
            Constraint::Feasible(value) => value.clone(),
            _ => panic!("Attempted to unwrap infeasible or undefined value."),
        }
    }

    pub fn unwrap_infeasible(&self) -> Y {
        match self {
            Constraint::Infeasible(value) => value.clone(),
            _ => panic!("Attempted to unwrap feasible or undefined value."),
        }
    }
}

impl<X: Display, Y: Display> Display for Constraint<X, Y> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Constraint::Feasible(val) => write!(f, "Feasible({})", val)?,
            Constraint::Infeasible(val) => write!(f, "Infeasible({})", val)?,
            Constraint::Undefined => write!(f, "Undefined")?,
        }

        Ok(())
    }
}
