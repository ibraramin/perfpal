/*
 * 02_string_building.c — quadratic string building and unbounded copies.
 *
 * Expected findings:
 *   PERF-C-STR-001   hot    strcat into a destination reused across iterations
 *   PERF-C-STR-002   warn   strlen() re-evaluated in a loop condition
 *   PERF-C-STR-003   warn   strcat/strcpy/gets/scanf("%s") unbounded family
 *                           (4 syntactic findings)
 *   PERF-C-STR-003   warn   identifier format argument — DOES NOT FIRE on the
 *                           default build; requires the semantic tier (below)
 *   PERF-C-LOOP-002  hot    printf() inside a loop
 *   PERF-C-LOOP-003  info   putc() as the primary statement of a loop body
 *                           (the fgetc in the while CONDITION does not fire)
 */

#include <stdio.h>
#include <string.h>

#include "perf_common.h"

static char log_buffer[4096];

/* Destination is rescanned from the start on every iteration => O(n^2). */
void append_all(const char **parts, int n)
{
    int i;

    log_buffer[0] = '\0';
    for (i = 0; i < n; i++) {
        strcat(log_buffer, parts[i]);       /* STR-001 + STR-003 */
        printf("%s\n", log_buffer);         /* LOOP-002 */
    }
}

/* strlen() is re-evaluated for every iteration (the string never changes). */
size_t scan_for_x(const char *text)
{
    size_t i;
    size_t hits = 0;

    for (i = 0; i < strlen(text); i++) {    /* STR-002 */
        if (text[i] == 'x') {
            hits++;
        }
    }
    return hits;
}

/* Unbounded reads: no length limit anywhere. */
void read_inputs(void)
{
    char name[64];
    char cmd[128];

    gets(cmd);                 /* STR-003 (gets) */
    scanf("%s", name);         /* STR-003 (scanf %s) */
    strcpy(name, cmd);         /* STR-003 (strcpy) */
    puts(name);
}

/* Format argument held in a variable: silent on the syntactic tier, resolved
 * by the semantic tier (single-assignment string-literal proof). */
void read_resolved(void)
{
    char word[64];
    const char *fmt = "%s";

    scanf(fmt, word);          /* STR-003 (semantic tier) */
    puts(word);
}

/* putc as the sole statement of the loop body. */
void echo_stream(FILE *in, FILE *out)
{
    int ch;

    while ((ch = fgetc(in)) != EOF) {
        putc(ch, out);         /* LOOP-003 */
    }
}
