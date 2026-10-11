"""EXECUTOR-001 executor creation per iteration."""

from concurrent.futures import ThreadPoolExecutor


def fan_out(tasks):
    for task in tasks:
        # TP PERF-PY-EXECUTOR-001 — a fresh executor every iteration.
        with ThreadPoolExecutor() as pool:
            pool.submit(task)


def fan_out_once(tasks):
    # CONTROL PERF-PY-EXECUTOR-001 — an executor created once is silent.
    with ThreadPoolExecutor() as pool:
        for task in tasks:
            pool.submit(task)
