# Precision corpus — Python string silent control (must stay silent).


def prefix_read(pieces):
    prefix = "["
    last = ""
    for piece in pieces:
        last = prefix + piece  # silent PERF-PY-STR-001 (not accumulated)
    return last
