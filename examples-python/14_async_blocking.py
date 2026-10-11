"""ASYNC-001 blocking call in `async def`; ASYNC-002 `await` inside a loop."""

import time

import requests


async def fetch_blocking(url):
    # TP PERF-PY-ASYNC-001 — a blocking HTTP client inside `async def`.
    return requests.get(url)


async def pause():
    # TP PERF-PY-ASYNC-001 — `time.sleep` blocks the single event loop.
    time.sleep(1)


def fetch_sync(url):
    # CONTROL PERF-PY-ASYNC-001 — the same blocking call in a sync def is silent.
    return requests.get(url)


async def await_each(urls):
    last = None
    # TP PERF-PY-ASYNC-002 — a sequential `await` inside a loop.
    for url in urls:
        last = await fetch_blocking(url)
    return last


async def await_once(url):
    # CONTROL PERF-PY-ASYNC-002 — an `await` outside every loop is silent.
    return await fetch_blocking(url)
