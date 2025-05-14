//! Process resource limits

use core::ops::{Index, IndexMut};

use linux_raw_sys::general::{RLIM_INFINITY, RLIM_NLIMITS, RLIMIT_STACK, rlimit64};

/// Process resource limits
pub struct Rlimits([rlimit64; RLIM_NLIMITS as usize]);

impl Default for Rlimits {
    fn default() -> Self {
        let mut result = Self(unsafe { core::mem::zeroed() });
        result[RLIMIT_STACK] = rlimit64 {
            rlim_cur: axconfig::plat::USER_STACK_SIZE as _,
            rlim_max: RLIM_INFINITY as _,
        };
        // TODO: do we need to set `RLIM_INFINITY` for other limits?
        result
    }
}

impl Index<u32> for Rlimits {
    type Output = rlimit64;
    fn index(&self, index: u32) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl IndexMut<u32> for Rlimits {
    fn index_mut(&mut self, index: u32) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}
