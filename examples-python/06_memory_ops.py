"""MEM-002 container copy inside a loop."""

import copy


def copy_each(rows):
    last = None
    # TP PERF-PY-MEM-002 — `copy.deepcopy` clones a whole container each pass.
    for row in rows:
        last = copy.deepcopy(row)
    return last


def copy_once(rows):
    # CONTROL PERF-PY-MEM-002 — copying outside every loop is silent.
    return copy.deepcopy(rows)
