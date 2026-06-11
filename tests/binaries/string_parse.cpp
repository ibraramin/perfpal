// string_parse.cpp — tutorial workload for perfpal.
//
// Two named hot functions doing string parsing. The point of this binary is
// to exercise a *different* user-function mix from hot_loop / alloc_thrash
// so the eval harness can verify the heuristics generalize.
//
// `parse_ints` walks a string and accumulates integer values.
// `count_words` walks a string and counts whitespace-separated tokens.
//
// Both functions do a comparable amount of work; expected ~50/50 split.
// `__attribute__((noinline))` keeps -O2 from collapsing them into main.
#include <cstdint>
#include <cstdlib>
#include <cctype>

static volatile uint64_t sink = 0;

__attribute__((noinline)) static void parse_ints(uint64_t n) {
    const size_t S = static_cast<size_t>(n) * 8 + 16;
    char* buf = new char[S];
    for (uint64_t i = 0; i < n; ++i) {
        char* p = buf + i * 8;
        int v = static_cast<int>(i & 0xffff);
        p[0] = '0' + (v / 1000) % 10;
        p[1] = '0' + (v / 100) % 10;
        p[2] = '0' + (v / 10) % 10;
        p[3] = '0' + v % 10;
        p[4] = ',';
        p[5] = ' ';
        p[6] = '0' + (v / 100) % 10;
        p[7] = ',';
    }
    buf[S - 1] = '\0';
    uint64_t total = 0;
    int cur = 0;
    bool in_num = false;
    for (size_t i = 0; buf[i]; ++i) {
        char c = buf[i];
        if (c >= '0' && c <= '9') {
            cur = cur * 10 + (c - '0');
            in_num = true;
        } else if (in_num) {
            total += static_cast<uint64_t>(cur);
            cur = 0;
            in_num = false;
        }
    }
    sink += total;
    delete[] buf;
}

__attribute__((noinline)) static void count_words(uint64_t n) {
    const size_t S = static_cast<size_t>(n) * 8 + 16;
    char* buf = new char[S];
    for (uint64_t i = 0; i < n; ++i) {
        char* p = buf + i * 8;
        p[0] = "ab"[i & 1];
        p[1] = "cd"[i & 1];
        p[2] = "ef"[i & 1];
        p[3] = "gh"[i & 1];
        p[4] = ' ';
        p[5] = "ij"[i & 1];
        p[6] = "kl"[i & 1];
        p[7] = (i & 1) ? ' ' : '\t';
    }
    buf[S - 1] = '\0';
    uint64_t words = 0;
    bool in_word = false;
    for (size_t i = 0; buf[i]; ++i) {
        unsigned char c = static_cast<unsigned char>(buf[i]);
        bool is_space = std::isspace(c);
        if (!is_space && !in_word) {
            words++;
            in_word = true;
        } else if (is_space) {
            in_word = false;
        }
    }
    sink += words;
    delete[] buf;
}

int main(int argc, char** argv) {
    uint64_t iters = 100'000'000;
    if (argc > 1) iters = std::strtoull(argv[1], nullptr, 10);
    parse_ints(iters);
    count_words(iters);
    return 0;
}
