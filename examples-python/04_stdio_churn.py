"""LOOP-002 expensive I/O inside a loop."""


def read_each(paths):
    total = 0
    # TP PERF-PY-LOOP-002 — `open(p)` performs file I/O every iteration.
    for p in paths:
        fh = open(p)
        total += 1
        del fh
    return total


def log_each(rows):
    for row in rows:
        # CONTROL PERF-PY-LOOP-002 — `print` is deliberately outside the I/O
        # family (72–83% of the C rule's raw hits).
        print(row)
