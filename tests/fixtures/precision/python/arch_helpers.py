# Precision corpus — Python cross-file callee definitions.


def py_unique_helper(value, lo, hi):
    """Uniquely defined corpus callee (the ARCH-003 true-positive target)."""
    if value < lo:
        return lo
    if value > hi:
        return hi
    return value
