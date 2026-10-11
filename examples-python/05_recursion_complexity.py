"""CALL family — PY-CALL-001 (direct recursion), PY-CALL-002 (complexity)."""


def walk_down(n):
    # TP PERF-PY-CALL-001 — `walk_down` calls itself directly.
    if n <= 0:
        return 0
    return walk_down(n - 1)


def step_once(n):
    # CONTROL PERF-PY-CALL-001 — a non-recursive function is silent.
    return n - 1


def branchy(value):
    # TP PERF-PY-CALL-002 — cyclomatic complexity well above the floor (15).
    score = 0
    if value > 1:
        score += 1
    if value > 2:
        score += 1
    if value > 3:
        score += 1
    if value > 4:
        score += 1
    if value > 5:
        score += 1
    if value > 6:
        score += 1
    if value > 7:
        score += 1
    if value > 8:
        score += 1
    if value > 9:
        score += 1
    if value > 10:
        score += 1
    if value > 11:
        score += 1
    if value > 12:
        score += 1
    if value > 13:
        score += 1
    if value > 14:
        score += 1
    if value > 15:
        score += 1
    if value > 16:
        score += 1
    return score


def simple(value):
    # CONTROL PERF-PY-CALL-002 — a low-complexity function is silent.
    return value + 1
