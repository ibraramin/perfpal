// branch_miss.cpp — tutorial workload for perfpal.
//
// Two named hot functions. `random_branch` dispatches on a pseudo-random
// value to produce branch-predictor misses. `sorted_branch` dispatches on a
// monotonically increasing value to keep the predictor happy. Both do
// comparable amounts of arithmetic work.
//
// `__attribute__((noinline))` keeps -O2 from collapsing them into main.
#include <cstdint>
#include <cstdlib>

static volatile uint64_t sink = 0;
static uint64_t xorshift_state = 0xcafef00d;

static uint64_t xorshift() {
    uint64_t x = xorshift_state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    xorshift_state = x;
    return x;
}

__attribute__((noinline)) static void random_branch(uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; ++i) {
        uint64_t v = xorshift();
        if (v & 1) {
            s += v * 3 + 1;
        } else {
            s += v * 2 - 1;
        }
    }
    sink += s;
}

__attribute__((noinline)) static void sorted_branch(uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; ++i) {
        if (i & 1) {
            s += i * 3 + 1;
        } else {
            s += i * 2 - 1;
        }
    }
    sink += s;
}

int main(int argc, char** argv) {
    uint64_t iters = 1'500'000'000;
    if (argc > 1) iters = std::strtoull(argv[1], nullptr, 10);
    random_branch(iters);
    sorted_branch(iters);
    return 0;
}
