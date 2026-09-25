/* Precision corpus — C string-accumulation silent control (must stay silent). */

#include <string.h>

/* SILENT STR-001: strcat outside any loop. */
void accumulate_once(char *dst, const char *src) {
    strcat(dst, src); /* silent PERF-C-STR-001 (outside loop) */
}
