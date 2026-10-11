"""LOOP-009 `try`/`except` inside a loop."""


def swallow(rows):
    ok = 0
    for row in rows:
        # TP PERF-PY-LOOP-009 — a `try` block inside the loop body.
        try:
            ok += int(row)
        except ValueError:
            pass
    return ok


def guarded(rows):
    # CONTROL PERF-PY-LOOP-009 — a `try` outside every loop is silent.
    try:
        return int(rows[0])
    except ValueError:
        return 0
