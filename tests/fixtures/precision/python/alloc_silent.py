# Precision corpus — Python alloc-family silent control (must stay silent).


def hoisted():
    return dict()  # silent PERF-PY-ALLOC-001 (constructor outside every loop)
