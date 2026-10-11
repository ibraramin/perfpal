"""PANDAS-001/002 row-wise iteration and `concat`-in-loop."""

import pandas as pd


def row_wise(df):
    total = 0
    # TP PERF-PY-PANDAS-001 — `iterrows` materialises a Series per row.
    for _, row in df.iterrows():
        total += row["x"]
    return total


def append_frames(frames):
    out = None
    # TP PERF-PY-PANDAS-002 — `pd.concat` rebuilds the frame every iteration.
    for frame in frames:
        out = pd.concat([out, frame])
    return out


def vectorised(df):
    # CONTROL PERF-PY-PANDAS-001 — a vectorised aggregation is not row-wise.
    return df["x"].mean()
