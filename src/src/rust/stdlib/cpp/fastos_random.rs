// ============================================================
// fastos_random.rs — <random> implementation
// ============================================================
// std::mt19937, distributions, random_device
// ============================================================

pub const RANDOM_TYPES: &[&str] = &[
    "mt19937", "mt19937_64", "minstd_rand", "minstd_rand0",
    "default_random_engine", "random_device",
    "uniform_int_distribution", "uniform_real_distribution",
    "normal_distribution", "bernoulli_distribution",
    "poisson_distribution", "exponential_distribution",
    "gamma_distribution", "weibull_distribution",
    "chi_squared_distribution", "cauchy_distribution",
    "fisher_f_distribution", "student_t_distribution",
    "discrete_distribution", "piecewise_constant_distribution",
    "piecewise_linear_distribution",
    "seed_seq",
];

pub const RANDOM_METHODS: &[&str] = &[
    "seed", "min", "max", "discard", "entropy",
];

pub fn is_random_symbol(name: &str) -> bool {
    RANDOM_TYPES.contains(&name) || RANDOM_METHODS.contains(&name)
}
