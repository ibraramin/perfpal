# Precision corpus — Python container copy (PERF-PY-MEM-002): planted true
# positive. Silent controls live in `mem_silent.py`.

import copy


def clone(rows):
    last = None
    for row in rows:
        last = copy.deepcopy(row)  # TP PERF-PY-MEM-002
    return last
