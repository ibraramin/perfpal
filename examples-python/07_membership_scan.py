"""LOOP-007 linear list scan inside a loop (flagship)."""


def contains_any(rows):
    table = [1, 2, 3]
    hits = 0
    # TP PERF-PY-LOOP-007 — `r in table` is an O(n) list scan per outer pass.
    for r in rows:
        if r in table:
            hits += 1
    return hits


def index_scan():
    data = [10, 20, 30]
    total = 0
    # TP PERF-PY-LOOP-007 — `data[i]` re-walks the list by position.
    for i in range(3):
        total += data[i]
    return total


def position_of(rows):
    data = [10, 20, 30]
    total = 0
    # TP PERF-PY-LOOP-007 — `data.index(r)` is an O(n) scan per iteration.
    for r in rows:
        total += data.index(r)
    return total


def set_membership(rows):
    seen = {1, 2, 3}
    for r in rows:
        # CONTROL PERF-PY-LOOP-007 — set membership is O(1) and excluded.
        if r in seen:
            return True
    return False
