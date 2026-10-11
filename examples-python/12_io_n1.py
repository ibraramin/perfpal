"""N1-001 DB/HTTP call inside a loop (N+1, flagship)."""


def fetch_each(conn, ids):
    last = None
    # TP PERF-PY-N1-001 — one DB round-trip per iteration (N+1).
    for i in ids:
        last = conn.execute(i)
    return last


def fetch_once(conn):
    # CONTROL PERF-PY-N1-001 — a DB call outside every loop is silent.
    return conn.execute("select 1")
