# Namespace package - allows multiple packages to share the 'horus' namespace
__path__ = __import__('pkgutil').extend_path(__path__, __name__)
