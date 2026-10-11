# Precision corpus — Python alloc family (PERF-PY-ALLOC-001): planted true
# positive. Silent controls live in `alloc_silent.py`.


def build(n):
    for _ in range(n):
        buf = dict()  # TP PERF-PY-ALLOC-001
        del buf
