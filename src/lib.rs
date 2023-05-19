use once_cell::sync::OnceCell;
use rand::SeedableRng;
use rand_xoshiro::rand_core::RngCore;
use rand_xoshiro::Xoshiro256PlusPlus;
use std::cell::UnsafeCell;
use std::rc::Rc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;

/// A global static seed value
static SEED: OnceCell<u64> = OnceCell::new();

/// An atomic counter that is incremented in each new thread
/// The random stream is jumped forward this many times
static STREAM_COUNTER: AtomicUsize = AtomicUsize::new(0);

// Copied from rand crate
#[derive(Clone, Debug)]
pub struct ThreadSmallRng {
    // Rc is explicitly !Send and !Sync
    rng: Rc<UnsafeCell<Xoshiro256PlusPlus>>,
}

thread_local!(
    static THREAD_SMALL_RNG_KEY: Rc<UnsafeCell<Xoshiro256PlusPlus>>  = {
        let seed = *SEED.get_or_init(|| 0);
        let rng = if seed != 0 {
            let mut proto = Xoshiro256PlusPlus::seed_from_u64(seed);
            for _ in 0..STREAM_COUNTER.fetch_add(1, SeqCst) {
                proto.jump();
            }
            proto
        } else {
            Xoshiro256PlusPlus::from_entropy()
        };
        Rc::new(UnsafeCell::new(rng))
    }
);

/// Sets the global seed for the random number generator
pub fn set_seed(seed: u64) {
    SEED.set(seed)
        .expect("Failed to set seed for random number generator");
}

/// Returns the global seed for the random number generator
pub fn get_seed() -> u64 {
    *SEED
        .get()
        .expect("Failed to fetch seed for random number generator")
}

pub fn get_count() -> usize {
    STREAM_COUNTER.load(SeqCst)
}

/// Returns a thread-local random number generator seeded with the global
/// seed but advanced so that each thread has an independent random stream.
/// If you create more than 2^64 threads or generate more than 2^64 random
/// numbers in any thread, then some of the values produced may be repeated.
/// Repeats are unlikey to influence the outcome of most non-crypto programs.
pub fn thread_small_rng() -> ThreadSmallRng {
    let rng = THREAD_SMALL_RNG_KEY.with(|t| t.clone());
    ThreadSmallRng { rng }
}

impl Default for ThreadSmallRng {
    fn default() -> ThreadSmallRng {
        thread_small_rng()
    }
}

impl RngCore for ThreadSmallRng {
    #[inline(always)]
    fn next_u32(&mut self) -> u32 {
        // SAFETY: We must make sure to stop using `rng` before anyone else
        // creates another mutable reference
        let rng = unsafe { &mut *self.rng.get() };
        rng.next_u32()
    }

    #[inline(always)]
    fn next_u64(&mut self) -> u64 {
        // SAFETY: We must make sure to stop using `rng` before anyone else
        // creates another mutable reference
        let rng = unsafe { &mut *self.rng.get() };
        rng.next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        // SAFETY: We must make sure to stop using `rng` before anyone else
        // creates another mutable reference
        let rng = unsafe { &mut *self.rng.get() };
        rng.fill_bytes(dest)
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        // SAFETY: We must make sure to stop using `rng` before anyone else
        // creates another mutable reference
        let rng = unsafe { &mut *self.rng.get() };
        rng.try_fill_bytes(dest)
    }
}

#[cfg(test)]
mod test {
    use std::thread;

    use crate::{get_count, get_seed, set_seed, thread_small_rng};
    use rand::Rng;
    use rayon::prelude::{IntoParallelIterator, ParallelIterator};

    #[test]
    fn test_thread_small_rng() {
        set_seed(1234);

        assert_eq!(get_seed(), 1234);
        assert_eq!(get_count(), 0);

        let v1: usize = thread::spawn(|| {
            assert_eq!(get_seed(), 1234);
            assert_eq!(get_count(), 0);
            thread_small_rng().gen()
        })
        .join()
        .unwrap();

        assert_eq!(get_count(), 1);

        let v2: usize = thread::spawn(|| {
            assert_eq!(get_seed(), 1234);
            assert_eq!(get_count(), 1);
            thread_small_rng().gen()
        })
        .join()
        .unwrap();

        assert_eq!(get_count(), 2);
        assert_eq!(get_seed(), 1234);

        assert_ne!(v1, v2);

        let rvals: Vec<u64> = (0..3)
            .into_par_iter()
            .map(|_| thread_small_rng().gen())
            .collect();
        assert_ne!(rvals[0], rvals[1]);
    }
}
