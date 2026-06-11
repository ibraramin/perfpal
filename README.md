# perfpal

Closed-source. See `LICENSE` for terms.

Local development:

    cargo build
    ./target/debug/perfpald diagnose --binary ./tests/binaries/hot_loop --duration 5

W1 eval gate:

    python3 eval/eval.py
