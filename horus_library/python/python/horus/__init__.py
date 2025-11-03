# HORUS namespace package - allows horus.library to coexist with main horus API
__path__ = __import__('pkgutil').extend_path(__path__, __name__)

# Load the horus_py __init__.py to get the main API (Node, Scheduler, etc.)
# This ensures that `import horus` gives you both horus.Node and horus.library
import os as _os
import sys as _sys

# Manually add horus_py path if it's not already in __path__
# This is needed because extend_path may not find it during pytest runs
_horus_py_candidates = [
    '/home/lord-patpak/horus/HORUS/horus_py/python/horus',
    _os.path.join(_os.path.dirname(_os.path.dirname(_os.path.dirname(_os.path.dirname(__file__)))), 'horus_py', 'python', 'horus'),
]
for _candidate in _horus_py_candidates:
    if _os.path.exists(_candidate) and _candidate not in __path__:
        __path__.append(_candidate)
        break

for _path in __path__:
    if 'horus_py' in _path:
        _init_file = _os.path.join(_path, '__init__.py')
        if _os.path.exists(_init_file):
            # Execute the horus_py __init__.py in this module's namespace
            with open(_init_file, 'r') as _f:
                exec(compile(_f.read(), _init_file, 'exec'), globals())
            break

# Clean up temporary variables
del _os, _sys, _path, _horus_py_candidates, _candidate
try:
    del _init_file, _f
except NameError:
    pass
