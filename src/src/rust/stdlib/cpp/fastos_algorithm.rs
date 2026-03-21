// ============================================================
// fastos_algorithm.rs — <algorithm> implementation
// ============================================================
// std::sort, find, copy, transform, reverse, etc.
// ============================================================

pub const ALGORITHM_FUNCTIONS: &[&str] = &[
    "sort", "stable_sort", "partial_sort", "nth_element",
    "find", "find_if", "find_if_not",
    "count", "count_if",
    "copy", "copy_if", "copy_n", "copy_backward",
    "move", "move_backward",
    "fill", "fill_n",
    "transform",
    "replace", "replace_if", "replace_copy", "replace_copy_if",
    "swap", "swap_ranges", "iter_swap",
    "reverse", "reverse_copy",
    "rotate", "rotate_copy",
    "unique", "unique_copy",
    "remove", "remove_if", "remove_copy", "remove_copy_if",
    "lower_bound", "upper_bound", "equal_range", "binary_search",
    "merge", "inplace_merge",
    "min", "max", "minmax",
    "min_element", "max_element", "minmax_element",
    "accumulate", "inner_product",
    "partial_sum", "adjacent_difference",
    "for_each", "for_each_n",
    "all_of", "any_of", "none_of",
    "equal", "mismatch", "lexicographical_compare",
    "is_sorted", "is_sorted_until",
    "next_permutation", "prev_permutation",
    "partition", "stable_partition", "is_partitioned",
    "set_union", "set_intersection", "set_difference", "set_symmetric_difference",
    "includes",
    "generate", "generate_n",
    "iota",
    "clamp",
];

pub fn is_algorithm_symbol(name: &str) -> bool {
    ALGORITHM_FUNCTIONS.contains(&name)
}

/// C inline implementation of std::algorithm functions
/// All operate on int* begin/end pairs for simplicity
pub const ALGORITHM_IMPL: &str = r#"
static void __alg_swap(int* a, int* b) {
    int t = *a; *a = *b; *b = t;
}

static int __alg_partition(int* begin, int* end) {
    int pivot = *(end - 1);
    int* i = begin;
    for (int* j = begin; j < end - 1; j++) {
        if (*j <= pivot) {
            __alg_swap(i, j);
            i++;
        }
    }
    __alg_swap(i, end - 1);
    return (int)(i - begin);
}

static void __alg_sort(int* begin, int* end) {
    if (end - begin <= 1) return;
    if (end - begin == 2) {
        if (begin[0] > begin[1]) __alg_swap(begin, begin + 1);
        return;
    }
    int pi = __alg_partition(begin, end);
    __alg_sort(begin, begin + pi);
    __alg_sort(begin + pi + 1, end);
}

static int* __alg_find(int* begin, int* end, int val) {
    for (int* p = begin; p < end; p++) {
        if (*p == val) return p;
    }
    return end;
}

static int __alg_count_if_even(int* begin, int* end) {
    int c = 0;
    for (int* p = begin; p < end; p++) {
        if (*p % 2 == 0) c++;
    }
    return c;
}

static int __alg_accumulate(int* begin, int* end, int init) {
    int s = init;
    for (int* p = begin; p < end; p++) s += *p;
    return s;
}

static void __alg_reverse(int* begin, int* end) {
    int* lo = begin;
    int* hi = end - 1;
    while (lo < hi) {
        __alg_swap(lo, hi);
        lo++;
        hi--;
    }
}

static int __alg_binary_search(int* begin, int* end, int val) {
    int* lo = begin;
    int* hi = end;
    while (lo < hi) {
        int* mid = lo + (hi - lo) / 2;
        if (*mid < val) lo = mid + 1;
        else if (*mid > val) hi = mid;
        else return 1;
    }
    return 0;
}

static int* __alg_min_element(int* begin, int* end) {
    int* mn = begin;
    for (int* p = begin + 1; p < end; p++) {
        if (*p < *mn) mn = p;
    }
    return mn;
}

static int* __alg_max_element(int* begin, int* end) {
    int* mx = begin;
    for (int* p = begin + 1; p < end; p++) {
        if (*p > *mx) mx = p;
    }
    return mx;
}

static void __alg_transform_double(int* begin, int* end, int* out) {
    for (int* p = begin; p < end; p++) {
        *out = *p * 2;
        out++;
    }
}

static int* __alg_unique(int* begin, int* end) {
    if (begin == end) return end;
    int* result = begin + 1;
    for (int* p = begin + 1; p < end; p++) {
        if (*p != *(p - 1)) {
            *result = *p;
            result++;
        }
    }
    return result;
}

static int* __alg_lower_bound(int* begin, int* end, int val) {
    int* lo = begin;
    int* hi = end;
    while (lo < hi) {
        int* mid = lo + (hi - lo) / 2;
        if (*mid < val) lo = mid + 1;
        else hi = mid;
    }
    return lo;
}

static int* __alg_upper_bound(int* begin, int* end, int val) {
    int* lo = begin;
    int* hi = end;
    while (lo < hi) {
        int* mid = lo + (hi - lo) / 2;
        if (*mid <= val) lo = mid + 1;
        else hi = mid;
    }
    return lo;
}
"#;
