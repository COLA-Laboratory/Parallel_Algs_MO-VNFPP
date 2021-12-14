pub fn round_to(num: f64, num_dp: usize) -> f64 {
    let ten: f64 = 10.0;
    let mult = ten.powf(num_dp as f64);
    (num * mult).round() / mult
}

// ----- Unit tests ---- //
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_round_to() {
        let pi = std::f64::consts::PI;

        let zero = round_to(pi, 0);
        let one = round_to(pi, 1);
        let two = round_to(pi, 2);
        let three = round_to(pi, 3);
        let four = round_to(pi, 4);

        assert_eq!(zero, 3.0); // (Pi is exactly 3!)
        assert_eq!(one, 3.1);
        assert_eq!(two, 3.14);
        assert_eq!(three, 3.142);
        assert_eq!(four, 3.1416);
    }
}
