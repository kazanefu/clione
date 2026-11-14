use std::time::Duration;

/// Real elapsed time for the current application update.
#[derive(Debug, Clone, Copy)]
pub struct Time {
    delta: Duration,
    elapsed: Duration,
}

impl Time {
    /// Creates a zeroed clock.
    pub const fn zero() -> Self {
        Self {
            delta: Duration::ZERO,
            elapsed: Duration::ZERO,
        }
    }

    /// Returns the elapsed time since the application started.
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Returns the duration since the previous application update.
    pub const fn delta(&self) -> Duration {
        self.delta
    }

    pub(crate) fn advance(&mut self, delta: Duration) {
        self.delta = delta;
        self.elapsed += delta;
    }
}

impl Default for Time {
    fn default() -> Self {
        Self::zero()
    }
}

/// Scaled and optionally paused time used by gameplay systems.
#[derive(Debug, Clone, Copy)]
pub struct VirtualTime {
    delta: Duration,
    elapsed: Duration,
    speed: f32,
    paused: bool,
}

impl VirtualTime {
    /// Creates an active clock with a speed of `1.0`.
    pub const fn new() -> Self {
        Self {
            delta: Duration::ZERO,
            elapsed: Duration::ZERO,
            speed: 1.0,
            paused: false,
        }
    }

    /// Returns the scaled duration for the current update.
    pub const fn delta(&self) -> Duration {
        self.delta
    }

    /// Returns the scaled elapsed time.
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Sets the time scale. Values below zero are treated as zero.
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.max(0.0);
    }

    /// Returns the current time scale.
    pub const fn speed(&self) -> f32 {
        self.speed
    }

    /// Pauses or resumes virtual time.
    pub const fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    /// Returns whether virtual time is paused.
    pub const fn is_paused(&self) -> bool {
        self.paused
    }

    pub(crate) fn advance(&mut self, real_delta: Duration) {
        self.delta = if self.paused {
            Duration::ZERO
        } else {
            real_delta.mul_f32(self.speed)
        };
        self.elapsed += self.delta;
    }
}

impl Default for VirtualTime {
    fn default() -> Self {
        Self::new()
    }
}

/// Fixed timestep configuration and accumulated time.
#[derive(Debug, Clone, Copy)]
pub struct FixedTime {
    step: Duration,
    accumulator: Duration,
    delta: Duration,
}

impl FixedTime {
    /// Creates a fixed clock with the supplied timestep.
    pub const fn new(step: Duration) -> Self {
        Self {
            step,
            accumulator: Duration::ZERO,
            delta: step,
        }
    }

    /// Returns the configured fixed timestep.
    pub const fn step(&self) -> Duration {
        self.step
    }

    /// Changes the fixed timestep.
    pub fn set_step(&mut self, step: Duration) {
        self.step = step.max(Duration::from_nanos(1));
    }

    /// Returns the timestep visible to FixedUpdate systems.
    pub const fn delta(&self) -> Duration {
        self.delta
    }

    pub(crate) fn accumulate(&mut self, delta: Duration) {
        self.accumulator += delta;
    }

    pub(crate) fn consume_step(&mut self) -> bool {
        if self.accumulator < self.step {
            return false;
        }
        self.accumulator -= self.step;
        self.delta = self.step;
        true
    }
}

impl Default for FixedTime {
    fn default() -> Self {
        Self::new(Duration::from_millis(16))
    }
}
