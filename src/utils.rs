use std::time;

#[must_use]
pub fn now() -> time::Instant {
	time::Instant::now()
}

#[must_use]
pub fn elapsed_secs(t0: &time::Instant) -> f32 {
	t0.elapsed().as_secs_f32()
}
