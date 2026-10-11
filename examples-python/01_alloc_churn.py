"""ALLOC family — PY-ALLOC-001 (loop-invariant construction), PY-ALLOC-003
(superlinear accumulation), PY-ALLOC-004 (eager materialisation)."""


def loop_invariant_construction(n):
    for _ in range(n):
        # TP PERF-PY-ALLOC-001 — `dict()` is constructed every iteration and
        # its value is independent of the loop variable.
        buf = dict()
        del buf


def hoisted_construction():
    # CONTROL PERF-PY-ALLOC-001 — a constructor outside every loop is silent.
    return dict()


def superlinear_concat(items):
    acc = []
    # TP PERF-PY-ALLOC-003 — non-in-place `acc = acc + [item]` rebuilds the
    # list each pass (O(n²) copying).
    for item in items:
        acc = acc + [item]
    return acc


def local_rebuild_is_benign(rows):
    count = 0
    for row in rows:
        # CONTROL PERF-PY-ALLOC-003 — `tmp` is rebuilt inside the loop, so it
        # never accumulates across iterations (the benign CTL-APPEND-LOCAL).
        tmp = []
        tmp.append(row)
        count += len(tmp)
    return count


def eager_materialise(rows):
    total = 0
    # TP PERF-PY-ALLOC-004 — `list(row)` forces an eager O(n) copy each pass.
    for row in rows:
        total += len(list(row))
    return total


def materialise_outside_loop(rows):
    # CONTROL PERF-PY-ALLOC-004 — materialising outside any loop is silent.
    return list(rows[0])
