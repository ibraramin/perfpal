# Precision corpus — Python container-copy silent control (must stay silent).

import copy


def clone_once(rows):
    return copy.deepcopy(rows)  # silent PERF-PY-MEM-002 (outside any loop)
