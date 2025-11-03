"""
Pytest configuration for HORUS Python tests.

Ensures test isolation by using unique session IDs for each test.
"""
import os
import uuid
import pytest


@pytest.fixture(autouse=True)
def unique_session_id():
    """
    Automatically set a unique HORUS_SESSION_ID for each test.

    This prevents hub exhaustion by ensuring each test uses its own
    shared memory namespace, avoiding conflicts between sequential tests.
    """
    # Generate a unique session ID for this test
    session_id = f"test_{uuid.uuid4().hex[:8]}"
    os.environ["HORUS_SESSION_ID"] = session_id

    yield session_id

    # Cleanup: Remove session ID after test
    if "HORUS_SESSION_ID" in os.environ:
        del os.environ["HORUS_SESSION_ID"]
