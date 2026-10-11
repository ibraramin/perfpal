"""STR family — PY-STR-001 (string accumulation in loop)."""


def build_string(pieces):
    s = ""
    # TP PERF-PY-STR-001 — `s += piece` accumulates across the loop.
    for piece in pieces:
        s += piece
    return s


def loop_invariant_prefix(pieces):
    prefix = "["
    last = ""
    for piece in pieces:
        # CONTROL PERF-PY-STR-001 — `prefix` is read, never rebound in the loop.
        last = prefix + piece
    return last
