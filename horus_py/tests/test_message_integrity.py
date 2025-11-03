"""
Message Integrity Tests - Python API via Rust Backend

Tests that verify all message types work correctly through the
Rust backend, ensuring data integrity across the Python-Rust boundary.
"""

import pytest
import horus
from horus.library import Pose2D, Twist, Transform, Point3, Vector3, Quaternion, CmdVel, LaserScan
import numpy as np


class TestPose2D:
    """Test Pose2D message integrity"""

    def test_pose2d_basic(self):
        """Test basic Pose2D creation and properties"""
        pose = Pose2D(x=1.5, y=2.5, theta=0.785)
        assert pose.x == 1.5
        assert pose.y == 2.5
        assert abs(pose.theta - 0.785) < 1e-10

    def test_pose2d_pubsub(self):
        """Test Pose2D through pub/sub"""
        test_values = []
        received_values = []

        def publisher(node):
            tick = node.info.tick_count()
            if tick < 5:
                x, y, theta = float(tick), float(tick) * 2.0, float(tick) * 0.5
                pose = Pose2D(x=x, y=y, theta=theta)
                node.send("pose_topic", pose)
                test_values.append((x, y, theta))
            else:
                node.request_stop()

        def subscriber(node):
            msg = node.get("pose_topic")
            if msg:
                received_values.append((msg.x, msg.y, msg.theta))
            if len(received_values) >= 5:
                node.request_stop()

        pub = horus.Node(name="pub", pubs="pose_topic", tick=publisher)
        sub = horus.Node(name="sub", subs="pose_topic", tick=subscriber)

        horus.run(pub, sub, duration=1.0)

        assert len(received_values) == 5
        for sent, received in zip(test_values, received_values):
            assert abs(sent[0] - received[0]) < 1e-10
            assert abs(sent[1] - received[1]) < 1e-10
            assert abs(sent[2] - received[2]) < 1e-10

    def test_pose2d_edge_cases(self):
        """Test Pose2D with edge case values"""
        edge_cases = [
            (0.0, 0.0, 0.0),
            (-100.0, -200.0, -3.14159),
            (1e6, 1e6, 6.28),
            (1e-10, 1e-10, 1e-10),
        ]

        for x, y, theta in edge_cases:
            pose = Pose2D(x=x, y=y, theta=theta)
            assert abs(pose.x - x) < 1e-9
            assert abs(pose.y - y) < 1e-9
            assert abs(pose.theta - theta) < 1e-9


class TestTwist:
    """Test Twist message integrity"""

    def test_twist_2d(self):
        """Test 2D twist creation"""
        twist = Twist.new_2d(linear_x=1.5, angular_z=0.5)
        assert twist.linear[0] == 1.5
        assert twist.angular[2] == 0.5

    def test_twist_pubsub(self):
        """Test Twist through pub/sub"""
        received = []

        def pub_node(node):
            if node.info.tick_count() == 0:
                twist = Twist.new_2d(linear_x=2.5, angular_z=1.2)
                node.send("twist_topic", twist)
            elif node.info.tick_count() >= 5:
                node.request_stop()

        def sub_node(node):
            msg = node.get("twist_topic")
            if msg:
                received.append((msg.linear[0], msg.angular[2]))
                node.request_stop()

        pub = horus.Node(name="pub", pubs="twist_topic", tick=pub_node)
        sub = horus.Node(name="sub", subs="twist_topic", tick=sub_node)

        horus.run(pub, sub, duration=1.0)

        assert len(received) > 0
        assert abs(received[0][0] - 2.5) < 1e-10
        assert abs(received[0][1] - 1.2) < 1e-10


class TestCmdVel:
    """Test CmdVel message integrity"""

    def test_cmdvel_basic(self):
        """Test CmdVel creation and properties"""
        cmd = CmdVel(linear=1.0, angular=0.5)
        assert abs(cmd.linear - 1.0) < 1e-6
        assert abs(cmd.angular - 0.5) < 1e-6

    def test_cmdvel_pubsub(self):
        """Test CmdVel through pub/sub with various values"""
        test_data = [
            (0.0, 0.0),
            (1.5, 0.5),
            (-0.5, -0.2),
            (2.0, 1.0),
        ]
        received_data = []

        def pub_node(node):
            tick = node.info.tick_count()
            if tick < len(test_data):
                linear, angular = test_data[tick]
                cmd = CmdVel(linear=linear, angular=angular)
                node.send("cmd_topic", cmd)
            else:
                node.request_stop()

        def sub_node(node):
            msg = node.get("cmd_topic")
            if msg:
                received_data.append((msg.linear, msg.angular))
            if len(received_data) >= len(test_data):
                node.request_stop()

        pub = horus.Node(name="pub", pubs="cmd_topic", tick=pub_node)
        sub = horus.Node(name="sub", subs="cmd_topic", tick=sub_node)

        horus.run(pub, sub, duration=1.0)

        assert len(received_data) == len(test_data)
        for sent, received in zip(test_data, received_data):
            assert abs(sent[0] - received[0]) < 1e-5
            assert abs(sent[1] - received[1]) < 1e-5


class TestLaserScan:
    """Test LaserScan message with NumPy integration"""

    def test_laserscan_numpy_getter(self):
        """Test LaserScan returns NumPy arrays"""
        scan = LaserScan()
        ranges = scan.ranges
        assert isinstance(ranges, np.ndarray)
        assert ranges.shape == (360,)
        assert ranges.dtype == np.float32

    def test_laserscan_numpy_setter(self):
        """Test LaserScan accepts NumPy arrays"""
        scan = LaserScan()
        test_ranges = np.random.rand(360).astype(np.float32) * 10.0
        scan.ranges = test_ranges

        retrieved = scan.ranges
        assert np.allclose(test_ranges, retrieved)

    def test_laserscan_list_setter(self):
        """Test LaserScan accepts Python lists (backward compat)"""
        scan = LaserScan()
        test_ranges = [float(i % 10) for i in range(360)]
        scan.ranges = test_ranges

        retrieved = scan.ranges
        assert np.allclose(test_ranges, retrieved)

    def test_laserscan_pubsub_numpy(self):
        """Test LaserScan with NumPy through pub/sub"""
        sent_ranges = None
        received_ranges = None

        def pub_node(node):
            nonlocal sent_ranges
            if node.info.tick_count() == 0:
                scan = LaserScan()
                sent_ranges = np.random.rand(360).astype(np.float32) * 10.0
                scan.ranges = sent_ranges
                node.send("scan_topic", scan)
            elif node.info.tick_count() >= 5:
                node.request_stop()

        def sub_node(node):
            nonlocal received_ranges
            msg = node.get("scan_topic")
            if msg:
                received_ranges = msg.ranges.copy()
                node.request_stop()

        pub = horus.Node(name="pub", pubs="scan_topic", tick=pub_node)
        sub = horus.Node(name="sub", subs="scan_topic", tick=sub_node)

        horus.run(pub, sub, duration=1.0)

        assert sent_ranges is not None
        assert received_ranges is not None
        assert np.allclose(sent_ranges, received_ranges)

    def test_laserscan_wrong_size_error(self):
        """Test LaserScan rejects wrong-sized arrays"""
        scan = LaserScan()

        with pytest.raises(ValueError, match="360"):
            scan.ranges = np.zeros(100, dtype=np.float32)

        with pytest.raises(ValueError, match="360"):
            scan.ranges = [1.0] * 100


class TestGeometricTypes:
    """Test Point3, Vector3, Quaternion, Transform"""

    def test_point3(self):
        """Test Point3 through pub/sub"""
        sent = []
        received = []

        def pub_node(node):
            if node.info.tick_count() == 0:
                p = Point3(x=1.5, y=2.5, z=3.5)
                node.send("point_topic", p)
                sent.append((p.x, p.y, p.z))
            elif node.info.tick_count() >= 5:
                node.request_stop()

        def sub_node(node):
            msg = node.get("point_topic")
            if msg:
                received.append((msg.x, msg.y, msg.z))
                node.request_stop()

        pub = horus.Node(name="pub", pubs="point_topic", tick=pub_node)
        sub = horus.Node(name="sub", subs="point_topic", tick=sub_node)

        horus.run(pub, sub, duration=1.0)

        assert len(received) > 0
        assert abs(sent[0][0] - received[0][0]) < 1e-10
        assert abs(sent[0][1] - received[0][1]) < 1e-10
        assert abs(sent[0][2] - received[0][2]) < 1e-10

    def test_vector3(self):
        """Test Vector3 operations"""
        v = Vector3(x=1.0, y=2.0, z=3.0)
        assert abs(v.magnitude() - np.sqrt(14.0)) < 1e-10

        v2 = Vector3(x=1.0, y=0.0, z=0.0)
        assert abs(v.dot(v2) - 1.0) < 1e-10

    def test_quaternion(self):
        """Test Quaternion creation"""
        q = Quaternion.identity()
        assert q.w == 1.0
        assert q.x == 0.0
        assert q.y == 0.0
        assert q.z == 0.0


class TestHighFrequency:
    """Test high-frequency communication"""

    def test_high_frequency_pose(self):
        """Test sustained high-rate Pose2D communication"""
        sent_count = [0]
        received_count = [0]

        def fast_pub(node):
            pose = Pose2D(x=float(sent_count[0]), y=0.0, theta=0.0)
            node.send("high_freq", pose)
            sent_count[0] += 1

            if sent_count[0] >= 100:
                node.request_stop()

        def fast_sub(node):
            msg = node.get("high_freq")
            if msg:
                received_count[0] += 1

        pub = horus.Node(name="pub", pubs="high_freq", tick=fast_pub)
        sub = horus.Node(name="sub", subs="high_freq", tick=fast_sub)

        horus.run(pub, sub, duration=1.0)

        # Should receive most messages
        reception_rate = received_count[0] / sent_count[0]
        assert reception_rate > 0.7, \
            f"Low reception rate: {reception_rate:.1%} ({received_count[0]}/{sent_count[0]})"


class TestErrorHandling:
    """Test error handling across Python-Rust boundary"""

    def test_error_recovery(self):
        """Test that errors don't crash the system"""
        error_count = [0]
        successful_ticks = [0]

        def faulty_node(node):
            if node.info.tick_count() % 3 == 0:
                raise ValueError("Intentional error")
            successful_ticks[0] += 1

        def error_handler(node, error):
            error_count[0] += 1

        node = horus.Node(name="faulty", tick=faulty_node, on_error=error_handler)
        horus.run(node, duration=0.5)

        assert error_count[0] > 0, "No errors were caught"
        assert successful_ticks[0] > 0, "No successful ticks"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
