"""COMPREHENSION-001 manual loop collapsible to a comprehension."""


def transform(row):
    return row * 2


def transform_all(rows):
    out = []
    # TP PERF-PY-COMPREHENSION-001 — the loop body is a single `append`.
    for row in rows:
        out.append(transform(row))
    return out


def accumulate_multi(rows):
    out = []
    for row in rows:
        # CONTROL PERF-PY-COMPREHENSION-001 — a multi-statement body is silent.
        value = transform(row)
        out.append(value)
    return out
