# thread_small_rng

This is a rust crate that enables multi-threaded Monte Carlo studies. It constructs a random number generator per thread and advances the random number generator so that each thread has access to an independent stream of random numbers. You can set a global seed for reproducible results or let each generator be seeded by system entropy. It will work conveniently with rayon. It is basically copied directly from the rand crate but with the addition of a global seed and a thread counter for jumping the generator. Pseudorandom numbers are generated using xoshiro256plusplus from the rand_xoshiro crate.
