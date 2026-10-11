# Precision corpus — Python accumulation (PERF-PY-ALLOC-003): planted true
# positive. Silent controls live in `alloc3_silent.py`.


def grow(items):
    acc = []
    for item in items:
        acc = acc + [item]  # TP PERF-PY-ALLOC-003
    return acc
