use std::time::{Duration, Instant};

#[derive(Copy, Clone)]
pub struct Stopwatch {
    duration: Duration,
    last_instant: Option<Instant>,
    is_running: bool,
}

impl Stopwatch {
    pub fn new() -> Stopwatch {
        Stopwatch {
            duration: Duration::from_secs(0),
            last_instant: None,
            is_running: false,
        }
    }

    pub fn start(&mut self) {
        if self.last_instant.is_none() {
            self.last_instant = Some(Instant::now());
        }

        self.update_time();
        self.is_running = true;
    }

    pub fn pause(&mut self) {
        self.update_time();
        self.is_running = false;
    }

    pub fn stop(mut self) -> u64 {
        self.update_time();
        let time = self.duration.as_secs();
        time
    }

    pub fn read(&mut self) -> u64 {
        self.update_time();
        let time = self.duration.as_secs();
        time
    }

    fn update_time(&mut self) {
        let last_instant = self.last_instant.unwrap();

        if self.is_running {
            self.duration += last_instant.elapsed();
        }

        self.last_instant = Some(Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_start_stop_read() {
        let mut test_watch = Stopwatch::new();

        test_watch.start();
        let start = test_watch.read();

        thread::sleep(Duration::from_secs(1));
        let read_one = test_watch.read();

        test_watch.pause();
        thread::sleep(Duration::from_secs(2));
        let read_two = test_watch.read();

        test_watch.start();
        thread::sleep(Duration::from_secs(2));
        let read_three = test_watch.read();

        assert_eq!(start, 0);
        assert!(read_one == 1);
        assert!(read_two == 1);
        assert!(read_three == 3);
    }
}
