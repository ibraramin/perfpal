# Precision corpus — Python string building (PERF-PY-STR-001): planted true
# positive. Silent controls live in `str_silent.py`.


def build(pieces):
    s = ""
    for piece in pieces:
        s += piece  # TP PERF-PY-STR-001
    return s
