"""Import-cycle participant A for PERF-PY-ARCH-001 (`pkg.cycle_a` <-> `pkg.cycle_b`)."""

from .cycle_b import ping_b


def walk_cycle(n):
    # TP PERF-PY-ARCH-001 — the cycle closes through `pkg.cycle_b`.
    return ping_b(n) + 1
