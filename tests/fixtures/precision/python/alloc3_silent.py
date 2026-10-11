# Precision corpus — Python accumulation silent control (must stay silent).


def rebuild(rows):
    count = 0
    for row in rows:
        tmp = []
        tmp.append(row)  # silent PERF-PY-ALLOC-003 (receiver rebuilt in loop)
        count += len(tmp)
    return count
