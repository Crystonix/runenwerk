use serde::{Deserialize, Serialize};

use crate::SimulationSeed;

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, runen_ecs::Component, runen_ecs::Resource,
)]
pub struct SimulationRng {
    state: u64,
    generated: u64,
}

impl Default for SimulationRng {
    fn default() -> Self {
        Self::from_seed(SimulationSeed::default())
    }
}

impl SimulationRng {
    pub fn from_seed(seed: SimulationSeed) -> Self {
        let state = if seed.0 == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed.0
        };
        Self {
            state,
            generated: 0,
        }
    }

    pub fn reseed(&mut self, seed: SimulationSeed) {
        *self = Self::from_seed(seed);
    }

    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        self.generated = self.generated.saturating_add(1);
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn next_f32(&mut self) -> f32 {
        let bits = (self.next_u64() >> 40) as u32;
        bits as f32 / (1u32 << 24) as f32
    }

    pub fn generated(&self) -> u64 {
        self.generated
    }
}
