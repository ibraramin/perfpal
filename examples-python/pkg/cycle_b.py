"""Import-cycle participant B for PERF-PY-ARCH-001 (`pkg.cycle_b` <-> `pkg.cycle_a`)."""

from .cycle_a import walk_cycle


def ping_b(n):
    # TP PERF-PY-ARCH-001 — the cycle closes through `pkg.cycle_a`.
    if n <= 0:
        return walk_cycle(0)
    return n
