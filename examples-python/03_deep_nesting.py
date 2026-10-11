"""LOOP-001 deep nesting (nesting reaches depth 3)."""


def triple_nested(a, b, c):
    total = 0
    for x in a:
        for y in b:
            # TP PERF-PY-LOOP-001 — this loop nests exactly 3 deep.
            for z in c:
                total += x + y + z
    return total


def double_nested(a, b):
    total = 0
    # CONTROL PERF-PY-LOOP-001 — depth 2 is below the floor (needs exactly 3).
    for x in a:
        for y in b:
            total += x + y
    return total
