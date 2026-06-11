// Tutorial workload: a known hot loop. The W1 prototype is just supposed to
// find the right function; W2 will add the memory-pressure heuristic.
#include <vector>
#include <algorithm>
#include <cstdint>
#include <cstdio>

// Intentional hot spot: a tight loop in a recognizable function.
__attribute__((noinline))
static uint64_t hot_loop(std::vector<int>& v) {
    uint64_t acc = 0;
    for (size_t i = 0; i + 1 < v.size(); ++i) {
        // std::sort is the real workhorse here; this loop is just a wrapper.
        if (v[i] > v[i + 1]) {
            std::swap(v[i], v[i + 1]);
            ++acc;
        }
    }
    return acc;
}

// A second function with a different name so the report has more than one entry.
__attribute__((noinline))
static uint64_t warm_loop(std::vector<int>& v) {
    uint64_t acc = 0;
    for (int x : v) {
        acc += static_cast<uint64_t>(x) * 31u;
    }
    return acc;
}

int main() {
    std::vector<int> v(200000);
    for (size_t i = 0; i < v.size(); ++i) {
        v[i] = static_cast<int>(v.size() - i);
    }
    uint64_t a = 0, b = 0;
    for (int it = 0; it < 2000; ++it) {
        a += hot_loop(v);
        b += warm_loop(v);
    }
    std::printf("acc=%llu %llu\n", (unsigned long long)a, (unsigned long long)b);
    return 0;
}
