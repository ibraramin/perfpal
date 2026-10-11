"""Perfpal Python demo corpus — package root (plan `python-backend`, W4).

A real package tree so the Python import resolver resolves the internal edges
that PERF-PY-ARCH-001 (import cycle) and PERF-PY-ARCH-003 (cross-module name
match) need. The numbered modules (``01_*`` … ``15_*``) are scanned by file
extension; they cannot be module-declared (Python identifiers cannot begin with
a digit). The ``pkg/`` subpackage holds the import-cycle pair (``cycle_a`` <->
``cycle_b``) and the cross-module helper.
"""
