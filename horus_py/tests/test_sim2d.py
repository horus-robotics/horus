"""Tests for sim2d Python API"""

import pytest


def test_import_sim2d():
    """Test that Sim2D can be imported"""
    from horus import Sim2D
    assert Sim2D is not None


def test_create_sim2d():
    """Test creating a Sim2D instance"""
    from horus import Sim2D

    sim = Sim2D(
        robot_name="test_robot",
        topic_prefix="test",
        headless=True,
        robot_width=0.5,
        robot_length=0.8,
        robot_max_speed=2.0,
        world_width=20.0,
        world_height=15.0
    )

    assert sim is not None
    assert "test_robot" in str(sim)
    assert "test" in str(sim)


def test_sim2d_add_obstacle():
    """Test adding obstacles to sim2d"""
    from horus import Sim2D

    sim = Sim2D(headless=True)

    # Add rectangle obstacle
    sim.add_obstacle(
        pos=(5.0, 5.0),
        size=(2.0, 1.0),
        shape="rectangle",
        color=(0.6, 0.4, 0.2)
    )

    # Add circle obstacle
    sim.add_obstacle(
        pos=(10.0, 10.0),
        size=(1.5, 1.5),
        shape="circle"
    )

    world_config = sim.get_world_config()
    assert world_config.obstacle_count == 2


def test_sim2d_clear_obstacles():
    """Test clearing obstacles"""
    from horus import Sim2D

    sim = Sim2D(headless=True)
    sim.add_obstacle((5.0, 5.0), (2.0, 1.0))
    sim.add_obstacle((10.0, 10.0), (1.5, 1.5))

    world_config = sim.get_world_config()
    assert world_config.obstacle_count == 2

    sim.clear_obstacles()
    world_config = sim.get_world_config()
    assert world_config.obstacle_count == 0


def test_sim2d_get_robot_config():
    """Test getting robot configuration"""
    from horus import Sim2D

    sim = Sim2D(
        robot_name="my_robot",
        topic_prefix="my_robot",
        headless=True,
        robot_width=0.6,
        robot_length=0.9,
        robot_max_speed=3.0
    )

    config = sim.get_robot_config()
    assert config.name == "my_robot"
    assert config.topic_prefix == "my_robot"
    assert config.width == pytest.approx(0.6, rel=1e-5)
    assert config.length == pytest.approx(0.9, rel=1e-5)
    assert config.max_speed == pytest.approx(3.0, rel=1e-5)
    assert len(config.color) == 3


def test_sim2d_get_world_config():
    """Test getting world configuration"""
    from horus import Sim2D

    sim = Sim2D(
        headless=True,
        world_width=30.0,
        world_height=25.0
    )

    config = sim.get_world_config()
    assert config.width == 30.0
    assert config.height == 25.0
    assert config.obstacle_count == 0


def test_sim2d_set_robot_position():
    """Test setting robot position"""
    from horus import Sim2D

    sim = Sim2D(headless=True)
    sim.set_robot_position((10.0, 5.0))

    # Position should be set in config
    # (actual position query would require running simulation)


def test_sim2d_set_robot_color():
    """Test setting robot color"""
    from horus import Sim2D

    sim = Sim2D(headless=True)
    sim.set_robot_color((1.0, 0.0, 0.0))  # Red

    config = sim.get_robot_config()
    assert config.color[0] == 1.0
    assert config.color[1] == 0.0
    assert config.color[2] == 0.0


def test_sim2d_invalid_obstacle_shape():
    """Test that invalid obstacle shape raises error"""
    from horus import Sim2D

    sim = Sim2D(headless=True)

    with pytest.raises(ValueError, match="rectangle.*circle"):
        sim.add_obstacle((5.0, 5.0), (2.0, 1.0), shape="triangle")


@pytest.mark.slow
def test_sim2d_run_simulation():
    """Test running simulation (slow test)"""
    from horus import Sim2D

    sim = Sim2D(headless=True)
    sim.add_obstacle((5.0, 5.0), (2.0, 1.0))

    # Run for 0.1 seconds (should complete quickly in headless mode)
    sim.run(duration=0.1)

    # If we get here, simulation ran without crashing


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
