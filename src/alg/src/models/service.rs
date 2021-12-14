use crate::operators::solution::Solution;
use std::fmt::Display;

pub type ServiceID = usize;

#[derive(Debug, Clone)]
pub struct Service {
    pub id: ServiceID,
    pub prod_rate: f64,
    pub vnfs: Vec<VNF>,
}

#[derive(Debug, Clone, Copy)]
pub struct VNF {
    pub service_rate: f64,
    pub queue_length: usize,
    pub size: usize,
}

impl Display for Solution<Option<&Service>> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for ch in &self.point {
            match ch {
                Some(service) => write!(f, "{},", service.id)?,
                None => write!(f, "_,")?,
            };
        }

        Ok(())
    }
}
