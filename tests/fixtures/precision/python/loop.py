# Precision corpus — Python list scan (PERF-PY-LOOP-007): planted true
# positive. Silent controls live in `loop_silent.py`.


def contains(rows):
    table = [1, 2, 3]
    hits = 0
    for r in rows:
        if r in table:  # TP PERF-PY-LOOP-007
            hits += 1
    return hits
