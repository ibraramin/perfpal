# Precision corpus — Python cross-module coupling silent control.


def wrapper(row):
    return same_file_only(row)  # silent PERF-PY-ARCH-003 (same-file def)


def same_file_only(row):
    return row
