"""LOOP-003 `range(len(seq))` iteration."""


def positional_sum(rows, seq):
    total = 0
    for _ in rows:
        # TP PERF-PY-LOOP-003 — `range(len(seq))` re-runs `len` each pass.
        for i in range(len(seq)):
            total += i
    return total


def counter_sum(n):
    total = 0
    # CONTROL PERF-PY-LOOP-003 — `range(n)` over a counter is not positional.
    for i in range(n):
        total += i
    return total
