use std::time;

/// Executes a given function at most once in the given time interval
///
/// The timer will execute slower than that interval
/// if the time between calls to Timer::tick() is longer than that interval
pub struct Timer {
    time_of_last_execution: time::Instant,
    interval: time::Duration,
    is_first_execution: bool,
}

impl Timer {
    pub fn new(interval: time::Duration) -> Self {
        Self {
            // this ensures that tick executes its function
            // on its first call
            time_of_last_execution: time::Instant::now(),
            interval,
            is_first_execution: true,
        }
    }

    /// Resets the timer
    pub fn reset(&mut self) {
        self.is_first_execution = true;
        self.time_of_last_execution = time::Instant::now();
    }

    /// Ticks the timer, executes the given function if the interval has been reached
    /// it is recommended to call Timer::reset() right before the first call to tick
    /// outside of the loop if its in one, which would look like this
    ///
    /// ```
    /// fn timer_example() {
    ///     // a timer that executes at most once every 10 milliseconds
    ///     let mut timer = Timer::new(Duration::from_millis(10))
    ///
    ///     timer.reset();
    ///     loop {
    ///         let should_execute = timer.tick();
    ///
    ///         if should_execute {
    ///             println!("Timer Tick!") // prints "Timer Tick!" at most every 10ms
    ///         }
    ///     }
    /// }
    /// ```
    pub fn tick(&mut self) -> bool {
        if self.is_first_execution {
            self.time_of_last_execution = time::Instant::now();
            self.is_first_execution = false;
            true
        } else {
            let now = time::Instant::now();
            let elapsed = now.duration_since(self.time_of_last_execution);

            if elapsed >= self.interval {
                // This takes into account starting the execution late and adjusts the last execution
                // time accordingly to try to be more accurate
                self.time_of_last_execution = now - (elapsed - self.interval);

                true
            } else {
                false
            }
        }
    }
}
