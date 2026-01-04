use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
use std::num::NonZeroU32;

pub struct IpRateLimiter {
    limiter: DefaultKeyedRateLimiter<String>,
}

impl IpRateLimiter {
    pub fn new_per_second(max: u32) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(max.max(1)).unwrap());
        let limiter = RateLimiter::keyed(quota);
        Self { limiter }
    }

    pub fn check(&self, key: String) -> bool {
        self.limiter.check_key(&key).is_ok()
    }
}
