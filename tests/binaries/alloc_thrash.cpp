// alloc_thrash.cpp — tutorial workload for perfpal.
//
// Two named hot functions doing equivalent work, plus a low-cost warm path.
// `alloc_thrash_loop` calls malloc+free per iteration (cache-pressure story).
// `flat_loop` does the same arithmetic with no allocations (compute-bound story).
//
// `__attribute__((noinline))` keeps -O2 from collapsing these into main; we
// want the named functions to show up in the profile so the eval harness can
// verify hotspot/bottleneck attribution end-to-end.
#include <cstdint>
#include <cstdlib>
#include <vector>

static volatile uint64_t sink = 0;

__attribute__((noinline)) static void alloc_thrash_loop(uint64_t n) {
    std::vector<int> v;
    v.reserve(64);
    for (uint64_t i = 0; i < n; ++i) {
        v.push_back(static_cast<int>(i & 0xff));
        if (v.size() == 64) {
            // Drain so capacity churns, exercising malloc/free.
            uint64_t s = 0;
            for (int x : v) s += x;
            sink += s;
            v.clear();
        }
    }
}

__attribute__((noinline)) static void flat_loop(uint64_t n) {
    uint64_t s = 0;
    for (uint64_t i = 0; i < n; ++i) {
        s += (i * 1103515245u + 12345u) & 0xffff;
    }
    sink += s;
}

int main(int argc, char** argv) {
    uint64_t iters = 5'000'000'000;
    if (argc > 1) iters = std::strtoull(argv[1], nullptr, 10);
    alloc_thrash_loop(iters);
    flat_loop(iters);
    return 0;
}
