//! 轻量随机数（xorshift64*）。
//!
//! 不引入 `rand`：程序只需要“均匀选一道菜”，
//! 自己实现几十行足够，而且方便用固定种子写可复现的测试。

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn seeded(seed: u64) -> Self {
        // 0 是 xorshift 的不动点，至少保证非零。
        Self {
            state: seed | 0x9E37_79B9_7F4A_7C15,
        }
    }

    pub fn from_time() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        Self::seeded(nanos)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// 返回 `0..n` 中的整数；`n == 0` 时返回 0（调用方保证非空）。
    pub fn gen_range(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u64() % n as u64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_with_same_seed() {
        let mut a = Rng::seeded(42);
        let mut b = Rng::seeded(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn range_is_bounded() {
        let mut rng = Rng::seeded(7);
        for _ in 0..1000 {
            let v = rng.gen_range(10);
            assert!(v < 10);
        }
    }
}
