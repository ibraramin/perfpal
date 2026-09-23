#ifndef E2E_MIXED_UTIL_H
#define E2E_MIXED_UTIL_H

/* Same-language cross-file target: defined in src/util.c, so src/main.c's
 * call emits PERF-C-ARCH-003 + a call_graph edge. */
int util_init(int n);

/* Cross-language target: defined in src/lib.rs. Declared here so the C
 * parser sees a well-formed call, but the language-scoped call-graph builder
 * deliberately never matches it against a C definition. */
int rust_surface(int n);

#endif
