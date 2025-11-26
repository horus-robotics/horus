"""
Tests for HORUS Python Hardware Nodes.

These tests run in simulation mode to verify node functionality
without requiring actual hardware.
"""

import pytest
import time
import threading
from unittest.mock import Mock, patch, MagicMock

# Import nodes module
from horus.nodes import (
    # Nodes
    SerialNode,
    JoystickNode,
    KeyboardNode,
    ImuNode,
    GpsNode,
    CameraNode,
    LidarNode,
    # Data types
    SerialData,
    JoystickState,
    KeyboardState,
    ImuData,
    GpsData,
    ImageData,
    LaserScan,
)


# =============================================================================
# Data Type Tests
# =============================================================================

class TestDataTypes:
    """Test data class functionality."""

    def test_serial_data_creation(self):
        """Test SerialData creation and defaults."""
        data = SerialData(port="/dev/ttyUSB0", data=b"Hello")
        assert data.port == "/dev/ttyUSB0"
        assert data.data == b"Hello"
        assert data.timestamp > 0

    def test_joystick_state_creation(self):
        """Test JoystickState creation and defaults."""
        state = JoystickState(
            axes=[0.0, 0.5, -0.5],
            buttons=[True, False, True],
            device_id=0,
        )
        assert len(state.axes) == 3
        assert state.axes[1] == 0.5
        assert state.buttons[0] == True
        assert state.device_id == 0

    def test_keyboard_state_creation(self):
        """Test KeyboardState creation."""
        state = KeyboardState(key='w', keycode=87, pressed=True)
        assert state.key == 'w'
        assert state.keycode == 87
        assert state.pressed == True

    def test_imu_data_creation(self):
        """Test ImuData creation with all fields."""
        data = ImuData(
            accel_x=0.0, accel_y=0.0, accel_z=9.81,
            gyro_x=0.01, gyro_y=0.02, gyro_z=0.03,
            temperature=25.0,
            frame_id="imu_link",
        )
        assert data.accel_z == 9.81
        assert data.gyro_x == 0.01
        assert data.temperature == 25.0
        assert data.frame_id == "imu_link"

    def test_gps_data_creation(self):
        """Test GpsData creation and has_fix method."""
        data = GpsData(
            latitude=37.7749,
            longitude=-122.4194,
            altitude=10.0,
            fix_type=1,
            satellites=8,
            hdop=1.2,
        )
        assert data.latitude == 37.7749
        assert data.longitude == -122.4194
        assert data.has_fix() == True

        # No fix case
        no_fix = GpsData(fix_type=0, satellites=0)
        assert no_fix.has_fix() == False

    def test_image_data_creation(self):
        """Test ImageData creation."""
        data = ImageData(
            data=b"\x00" * 100,
            width=10,
            height=10,
            encoding="bgr8",
            step=30,
        )
        assert data.width == 10
        assert data.height == 10
        assert data.encoding == "bgr8"
        assert len(data.data) == 100

    def test_laser_scan_creation(self):
        """Test LaserScan creation with defaults."""
        scan = LaserScan(
            ranges=[1.0, 2.0, 3.0],
            intensities=[10.0, 20.0, 30.0],
            angle_min=0.0,
            angle_max=3.14159,
            frame_id="laser",
        )
        assert len(scan.ranges) == 3
        assert len(scan.intensities) == 3
        assert scan.angle_min == 0.0
        assert scan.frame_id == "laser"
        assert scan.range_min == 0.1
        assert scan.range_max == 12.0


# =============================================================================
# SerialNode Tests
# =============================================================================

class TestSerialNode:
    """Test SerialNode functionality."""

    def test_serial_node_creation_simulation(self):
        """Test SerialNode creation in simulation mode."""
        node = SerialNode(
            port="/dev/ttyUSB0",
            baudrate=115200,
            simulation=True,
        )
        assert node.port == "/dev/ttyUSB0"
        assert node.baudrate == 115200
        assert node.simulation == True
        assert node.name.startswith("serial_")

    def test_serial_node_topics(self):
        """Test SerialNode topic configuration."""
        node = SerialNode(
            port="/dev/ttyUSB0",
            topic_prefix="my_serial",
            simulation=True,
        )
        assert "my_serial.rx" in node.pub_topics
        assert "my_serial.tx" in node.sub_topics

    def test_serial_node_init_simulation(self):
        """Test SerialNode initialization in simulation mode."""
        node = SerialNode(simulation=True)
        node._init(node)
        # Should not raise any errors
        assert node.simulation == True

    def test_serial_node_write_simulation(self):
        """Test SerialNode write in simulation mode."""
        node = SerialNode(simulation=True)
        node._init(node)

        result = node.write(b"Hello")
        assert result == True
        assert node.bytes_transmitted == 5

    def test_serial_node_transmit_various_types(self):
        """Test SerialNode transmit with various data types."""
        node = SerialNode(simulation=True)
        node._init(node)

        # bytes
        node._transmit(b"bytes")
        assert node.bytes_transmitted == 5

        # string
        node._transmit("string")
        assert node.bytes_transmitted == 11  # 5 + 6

        # SerialData
        serial_data = SerialData(port="test", data=b"test")
        node._transmit(serial_data)
        assert node.bytes_transmitted == 15  # 11 + 4


# =============================================================================
# JoystickNode Tests
# =============================================================================

class TestJoystickNode:
    """Test JoystickNode functionality."""

    def test_joystick_node_creation_simulation(self):
        """Test JoystickNode creation in simulation mode."""
        node = JoystickNode(
            device_id=0,
            deadzone=0.15,
            simulation=True,
        )
        assert node.device_id == 0
        assert node.deadzone == 0.15
        assert node.simulation == True

    def test_joystick_node_topics(self):
        """Test JoystickNode topic configuration."""
        node = JoystickNode(
            topic_prefix="gamepad",
            simulation=True,
        )
        assert "gamepad.state" in node.pub_topics
        assert "gamepad.axes" in node.pub_topics
        assert "gamepad.buttons" in node.pub_topics

    def test_joystick_node_tick_simulation(self):
        """Test JoystickNode tick in simulation mode publishes state."""
        node = JoystickNode(simulation=True)
        node._init(node)

        # Mock the send method
        sent_messages = []
        original_send = node.send
        def mock_send(topic, data):
            sent_messages.append((topic, data))
            return original_send(topic, data)
        node.send = mock_send

        node._tick(node)

        # Check that state was published
        topics = [msg[0] for msg in sent_messages]
        assert "joystick.state" in topics


# =============================================================================
# KeyboardNode Tests
# =============================================================================

class TestKeyboardNode:
    """Test KeyboardNode functionality."""

    def test_keyboard_node_creation_simulation(self):
        """Test KeyboardNode creation in simulation mode."""
        node = KeyboardNode(simulation=True)
        assert node.simulation == True
        assert node.name == "keyboard_input"

    def test_keyboard_node_topics(self):
        """Test KeyboardNode topic configuration."""
        node = KeyboardNode(
            topic_prefix="keys",
            simulation=True,
        )
        assert "keys.events" in node.pub_topics
        assert "keys.pressed" in node.pub_topics

    def test_keyboard_node_event_handling(self):
        """Test KeyboardNode internal event handling."""
        node = KeyboardNode(simulation=True)

        # Simulate key press event
        class MockKey:
            char = 'w'
            vk = 87

        node._on_press(MockKey())

        with node._lock:
            assert 'w' in node._pressed_keys
            assert len(node._events) == 1
            assert node._events[0].pressed == True

        # Simulate key release event
        node._on_release(MockKey())

        with node._lock:
            assert 'w' not in node._pressed_keys
            assert len(node._events) == 2
            assert node._events[1].pressed == False


# =============================================================================
# ImuNode Tests
# =============================================================================

class TestImuNode:
    """Test ImuNode functionality."""

    def test_imu_node_creation_simulation(self):
        """Test ImuNode creation in simulation mode."""
        node = ImuNode(
            i2c_bus=1,
            i2c_address=0x68,
            simulation=True,
        )
        assert node.i2c_bus == 1
        assert node.i2c_address == 0x68
        assert node.simulation == True

    def test_imu_node_frame_id(self):
        """Test ImuNode frame ID configuration."""
        node = ImuNode(
            frame_id="my_imu",
            simulation=True,
        )
        assert node.frame_id == "my_imu"

    def test_imu_node_tick_simulation(self):
        """Test ImuNode tick in simulation mode publishes data."""
        node = ImuNode(simulation=True)
        node._init(node)

        # Mock the send method
        sent_messages = []
        def mock_send(topic, data):
            sent_messages.append((topic, data))
            return True
        node.send = mock_send

        node._tick(node)

        assert len(sent_messages) == 1
        assert sent_messages[0][0] == "imu"
        assert isinstance(sent_messages[0][1], ImuData)
        # Check gravity simulation
        assert abs(sent_messages[0][1].accel_z - 9.81) < 0.1

    def test_imu_node_to_signed_16bit(self):
        """Test ImuNode 16-bit conversion."""
        # Positive value
        assert ImuNode._to_signed_16bit(0x00, 0x64) == 100
        # Negative value
        assert ImuNode._to_signed_16bit(0xFF, 0x9C) == -100
        # Zero
        assert ImuNode._to_signed_16bit(0x00, 0x00) == 0
        # Max positive
        assert ImuNode._to_signed_16bit(0x7F, 0xFF) == 32767
        # Min negative
        assert ImuNode._to_signed_16bit(0x80, 0x00) == -32768


# =============================================================================
# GpsNode Tests
# =============================================================================

class TestGpsNode:
    """Test GpsNode functionality."""

    def test_gps_node_creation_simulation(self):
        """Test GpsNode creation in simulation mode."""
        node = GpsNode(
            port="/dev/ttyUSB0",
            baudrate=9600,
            simulation=True,
        )
        assert node.port == "/dev/ttyUSB0"
        assert node.baudrate == 9600
        assert node.simulation == True

    def test_gps_node_topics(self):
        """Test GpsNode topic configuration."""
        node = GpsNode(
            topic_prefix="gnss",
            simulation=True,
        )
        assert "gnss.fix" in node.pub_topics

    def test_gps_node_tick_simulation(self):
        """Test GpsNode tick in simulation mode publishes data."""
        node = GpsNode(simulation=True)
        node._init(node)

        # Mock the send method
        sent_messages = []
        def mock_send(topic, data):
            sent_messages.append((topic, data))
            return True
        node.send = mock_send

        node._tick(node)

        assert len(sent_messages) == 1
        assert sent_messages[0][0] == "gps.fix"
        data = sent_messages[0][1]
        assert isinstance(data, GpsData)
        # Check simulated position is near San Francisco
        assert abs(data.latitude - 37.7749) < 0.01
        assert abs(data.longitude - (-122.4194)) < 0.01

    def test_gps_node_last_fix_property(self):
        """Test GpsNode last_fix property."""
        node = GpsNode(simulation=True)
        assert node.fix_count == 0
        assert isinstance(node.last_fix, GpsData)


# =============================================================================
# CameraNode Tests
# =============================================================================

class TestCameraNode:
    """Test CameraNode functionality."""

    def test_camera_node_creation_simulation(self):
        """Test CameraNode creation in simulation mode."""
        node = CameraNode(
            device_id=0,
            width=640,
            height=480,
            fps=30.0,
            simulation=True,
        )
        assert node.device_id == 0
        assert node.width == 640
        assert node.height == 480
        assert node.fps == 30.0
        assert node.simulation == True

    def test_camera_node_topics(self):
        """Test CameraNode topic configuration."""
        node = CameraNode(
            topic_prefix="webcam",
            simulation=True,
        )
        assert "webcam.image" in node.pub_topics
        assert "webcam.image_raw" in node.pub_topics

    def test_camera_node_tick_simulation(self):
        """Test CameraNode tick in simulation mode publishes frame."""
        node = CameraNode(
            width=320,
            height=240,
            simulation=True,
        )
        node._init(node)

        # Mock the send method
        sent_messages = []
        def mock_send(topic, data):
            sent_messages.append((topic, data))
            return True
        node.send = mock_send

        node._tick(node)

        assert len(sent_messages) == 2  # raw + image
        assert node.frame_count == 1

        # Check ImageData
        image_msgs = [m for m in sent_messages if m[0].endswith(".image")]
        assert len(image_msgs) == 1
        image_data = image_msgs[0][1]
        assert isinstance(image_data, ImageData)
        assert image_data.width == 320
        assert image_data.height == 240

    def test_camera_node_frame_count(self):
        """Test CameraNode frame counter."""
        node = CameraNode(simulation=True)
        node._init(node)

        # Suppress actual send
        node.send = lambda topic, data: True

        assert node.frame_count == 0
        node._tick(node)
        assert node.frame_count == 1
        node._tick(node)
        assert node.frame_count == 2


# =============================================================================
# LidarNode Tests
# =============================================================================

class TestLidarNode:
    """Test LidarNode functionality."""

    def test_lidar_node_creation(self):
        """Test LidarNode creation in simulation mode."""
        node = LidarNode(
            port="/dev/ttyUSB0",
            num_samples=360,
            range_min=0.15,
            range_max=12.0,
            simulation=True,
        )
        assert node.num_samples == 360
        assert node.range_min == 0.15
        assert node.range_max == 12.0
        assert node.simulation == True

    def test_lidar_node_tick(self):
        """Test LidarNode tick publishes LaserScan data."""
        node = LidarNode(num_samples=36, simulation=True)
        node._init(node)

        # Mock the send method
        sent_messages = []
        def mock_send(topic, data):
            sent_messages.append((topic, data))
            return True
        node.send = mock_send

        node._tick(node)

        assert len(sent_messages) == 1
        assert sent_messages[0][0] == "scan"
        data = sent_messages[0][1]
        assert isinstance(data, LaserScan)
        assert len(data.ranges) == 36
        assert len(data.intensities) == 36
        assert data.angle_min == 0.0
        assert data.angle_max > 6.0  # ~2*pi

    def test_lidar_node_scan_count(self):
        """Test LidarNode scan counter."""
        node = LidarNode(simulation=True)
        node._init(node)
        node.send = lambda topic, data: True

        assert node.scan_count == 0
        node._tick(node)
        assert node.scan_count == 1
        node._tick(node)
        assert node.scan_count == 2


# =============================================================================
# Integration Tests
# =============================================================================

class TestNodeIntegration:
    """Integration tests for node lifecycle."""

    def test_node_lifecycle(self):
        """Test full node lifecycle: init -> tick -> shutdown."""
        node = SerialNode(simulation=True)

        # Init
        node._init(node)
        assert node.simulation == True

        # Tick
        node._tick(node)

        # Shutdown
        node._shutdown(node)
        # Should not raise

    def test_multiple_nodes_simulation(self):
        """Test creating multiple nodes in simulation mode."""
        serial = SerialNode(simulation=True, name="serial")
        imu = ImuNode(simulation=True, name="imu")
        gps = GpsNode(simulation=True, name="gps")
        camera = CameraNode(simulation=True, name="camera")

        # All should have unique names
        names = {serial.name, imu.name, gps.name, camera.name}
        assert len(names) == 4

        # All should be in simulation mode
        assert all(n.simulation for n in [serial, imu, gps, camera])


# =============================================================================
# Run Tests
# =============================================================================

if __name__ == "__main__":
    pytest.main([__file__, "-v"])
