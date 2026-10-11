"""Cross-module helpers for PERF-PY-ARCH-003 (name-based call coupling).

``clamp_value`` is defined here and called by bare name from
``15_cross_file_and_arch.py`` — the duck-typed cross-module call edge the rule
resolves by name.
"""


def clamp_value(value, lo, hi):
    if value < lo:
        return lo
    if value > hi:
        return hi
    return value


def normalize(value):
    return clamp_value(value, 0, 100)
