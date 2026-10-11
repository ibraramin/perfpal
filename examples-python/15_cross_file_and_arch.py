"""ARCH-003 name-based cross-module call coupling."""

from pkg.arch_helpers import clamp_value


def use_cross_file(raw):
    # TP PERF-PY-ARCH-003 — `clamp_value` is defined only in pkg/arch_helpers.py.
    return clamp_value(raw, 0, 100)


def local_helper(row):
    # CONTROL PERF-PY-ARCH-003 — a same-file helper has no cross-file def.
    return local_only(row)


def local_only(row):
    return row
