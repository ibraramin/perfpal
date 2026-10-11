"""Re-export hub for the demo subpackage (plan `python-backend`, W4).

``__init__.py`` re-export handling is decided explicitly: a hub's own relative
edges resolve to the owning modules; deep name-level re-export tracing is out
of scope (T2 `python-resolver`) and is deliberately not guessed here.
"""

from .cycle_a import walk_cycle
from .cycle_b import ping_b

__all__ = ["ping_b", "walk_cycle"]
