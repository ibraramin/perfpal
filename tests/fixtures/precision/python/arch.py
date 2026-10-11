# Precision corpus — Python cross-module call coupling (PERF-PY-ARCH-003):
# planted true positive. Silent controls live in `arch_silent.py`.


def use(raw):
    return py_unique_helper(raw, 0, 100)  # TP PERF-PY-ARCH-003
