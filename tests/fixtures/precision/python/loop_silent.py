# Precision corpus — Python list-scan silent control (must stay silent).


def in_set(rows):
    seen = {1, 2, 3}
    for r in rows:
        if r in seen:  # silent PERF-PY-LOOP-007 (set/dict container excluded)
            return True
    return False
