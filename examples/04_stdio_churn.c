/*
 * 04_stdio_churn.c — per-byte and per-iteration stdio work.
 *
 * Expected findings:
 *   PERF-C-LOOP-003  info   putc()/fputc() as primary loop statements (2 sites).
 *                           The fgetc()/getc() calls sit in while CONDITIONS and
 *                           do not fire.
 *   PERF-C-LOOP-002  hot    fopen() and sprintf(), both unguarded, in loops
 *   PERF-C-LOOP-002  warn   fprintf() and fclose() inside the guarded branch
 *   PERF-C-STR-003   warn   unbounded sprintf
 */

#include <stdio.h>
#include <stdlib.h>

#include "perf_common.h"

/* Byte-at-a-time copy: one libc call (and lock) per character. */
void copy_charwise(FILE *in, FILE *out)
{
    int ch;

    while ((ch = fgetc(in)) != EOF) {
        putc(ch, out);                     /* LOOP-003 (primary statement) */
    }
}

/* Block reads are the standard fix for the loop above. */
void copy_blockwise(FILE *in, FILE *out)
{
    char chunk[4096];
    size_t got;

    while ((got = fread(chunk, 1, sizeof chunk, in)) > 0) {
        fwrite(chunk, 1, got, out);
    }
}

/* One open/format/close cycle per iteration: syscall churn. */
void touch_many(const char **names, int n)
{
    int i;

    for (i = 0; i < n; i++) {
        FILE *fp = fopen(names[i], "w");   /* LOOP-002 (hot) */
        if (fp != NULL) {
            fprintf(fp, "%d\n", i);        /* LOOP-002 (guarded -> warn) */
            fclose(fp);                    /* LOOP-002 (guarded -> warn) */
        }
    }
}

/* getc/fputc inside a loop, with a conditional guard. */
int strip_controls(FILE *fp)
{
    int ch;
    int kept = 0;

    while ((ch = getc(fp)) != EOF) {
        if (ch < 32) {
            continue;
        }
        fputc(ch, stdout);                 /* LOOP-003 */
        kept++;
    }
    return kept;
}

/* Repeated formatting into a fixed buffer inside a loop. */
void render_report(const char **rows, int n)
{
    char line[256];
    int i;

    for (i = 0; i < n; i++) {
        sprintf(line, "[%d] %s", i, rows[i]);   /* STR-003 + LOOP-002 */
        puts(line);
    }
}
