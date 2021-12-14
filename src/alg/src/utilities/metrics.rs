pub fn mean(x: &Vec<f64>) -> f64 {
    x.into_iter().sum::<f64>() / x.len() as f64
}

pub fn cum_moving_avg(current_mean: f64, new_value: f64, num_points: usize) -> f64 {
    current_mean + (new_value - current_mean) / (num_points + 1) as f64
}

pub fn variance(x: &Vec<f64>) -> f64 {
    let mean_x = mean(x);
    x.iter().map(|x| (x - mean_x).powf(2.0)).sum()
}

// Finds the sample covariance between two series
// Assumes that the indexes of each vector align
pub fn cov(x: &Vec<f64>, y: &Vec<f64>) -> f64 {
    let mean_x = mean(x);
    let mean_y = mean(y);

    x.iter()
        .zip(y)
        .map(|(x, y)| (x - mean_x) * (y - mean_y))
        .sum()
}

// Finds the correlation between two series
// Assumes that the indexes of each vector align
pub fn pearson_correlation(x: &Vec<f64>, y: &Vec<f64>) -> f64 {
    let mean_x = mean(x);
    let mean_y = mean(y);

    let cov: f64 = x
        .iter()
        .zip(y)
        .map(|(x, y)| (x - mean_x) * (y - mean_y))
        .sum();

    // Leaving SQRT till the end to minimise floating point errors
    let variance_x: f64 = x.iter().map(|x| (x - mean_x).powf(2.0)).sum();
    let variance_y: f64 = y.iter().map(|y| (y - mean_y).powf(2.0)).sum();

    cov / (variance_x * variance_y).sqrt()
}

// ----- Unit tests ---- //
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pearson_correlation() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b = vec![1.2, 2.1, 3.3, 3.9, 4.8]; // Less similar
        let c = vec![0.5, 5.5, 1.8, 4.2, 1.0]; // Not similar
        let d = vec![5.0, 4.0, 3.0, 2.0, 1.0]; // Opposite
        let e = vec![100.0, 200.0, 300.0, 400.0, 500.0]; // Scaled

        let equal = pearson_correlation(&a, &a);
        let similar = pearson_correlation(&a, &b);
        let dissimilar = pearson_correlation(&a, &c);
        let opposite = pearson_correlation(&a, &d);
        let scaled = pearson_correlation(&a, &e);

        assert_eq!(equal, 1.0);
        assert!(similar > 0.9955 && similar < 0.9957);
        assert!(dissimilar > -0.0235 && dissimilar < -0.0215);
        assert!(opposite > -1.01 && opposite < -0.99);
        assert_eq!(scaled, 1.0);
    }
}
