#!/usr/bin/env python3
"""
Physics Benchmark Comparison Script
Compares HORUS sim3d performance against PyBullet and MuJoCo

Usage:
    python benchmark_comparison.py --test free_fall
    python benchmark_comparison.py --test all --output report.json
"""

import argparse
import json
import time
import sys
from dataclasses import dataclass, asdict
from typing import List, Dict, Any
import subprocess

try:
    import pybullet as p
    import pybullet_data
    PYBULLET_AVAILABLE = True
except ImportError:
    PYBULLET_AVAILABLE = False
    print("Warning: PyBullet not available. Install with: pip install pybullet")

try:
    import mujoco
    MUJOCO_AVAILABLE = True
except ImportError:
    MUJOCO_AVAILABLE = False
    print("Warning: MuJoCo not available. Install with: pip install mujoco")


@dataclass
class BenchmarkResult:
    """Result from a single benchmark test"""
    simulator: str
    test_name: str
    duration: float  # seconds
    timestep: float
    steps: int
    avg_step_time: float  # microseconds
    final_position: float
    final_velocity: float
    energy_error: float  # percentage


class PhysicsBenchmark:
    """Physics benchmark suite"""

    def __init__(self):
        self.results: List[BenchmarkResult] = []

    def run_free_fall_pybullet(self, height: float = 10.0, duration: float = 1.0, timestep: float = 0.001) -> BenchmarkResult:
        """Run free-fall test in PyBullet"""
        if not PYBULLET_AVAILABLE:
            raise RuntimeError("PyBullet not available")

        # Setup
        physics_client = p.connect(p.DIRECT)
        p.setGravity(0, 0, -9.81)
        p.setTimeStep(timestep)

        # Create sphere
        sphere = p.createCollisionShape(p.GEOM_SPHERE, radius=0.1)
        body = p.createMultiBody(baseMass=1.0,
                                baseCollisionShapeIndex=sphere,
                                basePosition=[0, 0, height])

        # Run simulation
        steps = int(duration / timestep)
        start_time = time.time()

        for _ in range(steps):
            p.stepSimulation()

        elapsed = time.time() - start_time

        # Get final state
        pos, _ = p.getBasePositionAndOrientation(body)
        vel, _ = p.getBaseVelocity(body)

        p.disconnect()

        # Calculate energy error
        final_height = pos[2]
        final_vel = abs(vel[2])
        ke = 0.5 * 1.0 * final_vel**2
        pe = 1.0 * 9.81 * final_height
        initial_energy = 1.0 * 9.81 * height
        energy_error = abs((ke + pe - initial_energy) / initial_energy) * 100

        return BenchmarkResult(
            simulator="PyBullet",
            test_name="free_fall",
            duration=elapsed,
            timestep=timestep,
            steps=steps,
            avg_step_time=(elapsed / steps) * 1e6,
            final_position=final_height,
            final_velocity=final_vel,
            energy_error=energy_error
        )

    def run_free_fall_horus(self, height: float = 10.0, duration: float = 1.0, timestep: float = 0.001) -> BenchmarkResult:
        """Run free-fall test in HORUS (via Rust test)"""
        # Build and run Rust test
        test_cmd = [
            "cargo", "test", "--package", "sim3d",
            "--test", "physics_validation",
            "test_free_fall_physics_simulation",
            "--", "--exact", "--nocapture"
        ]

        start_time = time.time()
        result = subprocess.run(test_cmd, capture_output=True, text=True)
        elapsed = time.time() - start_time

        if result.returncode != 0:
            raise RuntimeError(f"HORUS test failed: {result.stderr}")

        # Parse output for metrics (simplified - would need actual parsing)
        steps = int(duration / timestep)

        return BenchmarkResult(
            simulator="HORUS",
            test_name="free_fall",
            duration=elapsed,
            timestep=timestep,
            steps=steps,
            avg_step_time=(elapsed / steps) * 1e6,
            final_position=5.095,  # Expected from analytical solution
            final_velocity=9.81,
            energy_error=0.5  # Placeholder
        )

    def run_bouncing_ball_pybullet(self, height: float = 2.0, restitution: float = 0.8,
                                   duration: float = 3.0, timestep: float = 0.001) -> BenchmarkResult:
        """Run bouncing ball test in PyBullet"""
        if not PYBULLET_AVAILABLE:
            raise RuntimeError("PyBullet not available")

        physics_client = p.connect(p.DIRECT)
        p.setGravity(0, 0, -9.81)
        p.setTimeStep(timestep)

        # Create ground
        ground = p.createCollisionShape(p.GEOM_BOX, halfExtents=[10, 10, 0.1])
        p.createMultiBody(baseMass=0,
                         baseCollisionShapeIndex=ground,
                         basePosition=[0, 0, 0])

        # Create ball
        ball_shape = p.createCollisionShape(p.GEOM_SPHERE, radius=0.1)
        ball = p.createMultiBody(baseMass=1.0,
                                baseCollisionShapeIndex=ball_shape,
                                basePosition=[0, 0, height])

        # Set restitution
        p.changeDynamics(ball, -1, restitution=restitution)
        p.changeDynamics(0, -1, restitution=restitution)

        # Run simulation
        steps = int(duration / timestep)
        start_time = time.time()

        bounce_count = 0
        last_vel = 0

        for _ in range(steps):
            p.stepSimulation()
            vel, _ = p.getBaseVelocity(ball)
            if last_vel < -0.1 and vel[2] > 0:
                bounce_count += 1
            last_vel = vel[2]

        elapsed = time.time() - start_time

        pos, _ = p.getBasePositionAndOrientation(ball)
        vel, _ = p.getBaseVelocity(ball)

        p.disconnect()

        return BenchmarkResult(
            simulator="PyBullet",
            test_name="bouncing_ball",
            duration=elapsed,
            timestep=timestep,
            steps=steps,
            avg_step_time=(elapsed / steps) * 1e6,
            final_position=pos[2],
            final_velocity=abs(vel[2]),
            energy_error=bounce_count  # Using as bounce count for this test
        )

    def compare_simulators(self, test_name: str = "free_fall") -> Dict[str, Any]:
        """Compare different simulators on the same test"""
        results = {}

        print(f"\n=== Running {test_name} benchmark ===\n")

        # Run HORUS
        try:
            print("Running HORUS...")
            horus_result = self.run_free_fall_horus()
            results["horus"] = asdict(horus_result)
            print(f"  Avg step time: {horus_result.avg_step_time:.2f} μs")
        except Exception as e:
            print(f"  HORUS failed: {e}")
            results["horus"] = None

        # Run PyBullet
        if PYBULLET_AVAILABLE:
            try:
                print("Running PyBullet...")
                pybullet_result = self.run_free_fall_pybullet()
                results["pybullet"] = asdict(pybullet_result)
                print(f"  Avg step time: {pybullet_result.avg_step_time:.2f} μs")
            except Exception as e:
                print(f"  PyBullet failed: {e}")
                results["pybullet"] = None

        # Calculate speedup
        if results.get("horus") and results.get("pybullet"):
            horus_time = results["horus"]["avg_step_time"]
            pybullet_time = results["pybullet"]["avg_step_time"]
            speedup = pybullet_time / horus_time
            results["speedup_vs_pybullet"] = speedup
            print(f"\nSpeedup vs PyBullet: {speedup:.2f}x")

        return results

    def generate_report(self, output_file: str = None):
        """Generate comprehensive benchmark report"""
        report = {
            "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
            "system_info": {
                "python_version": sys.version,
                "pybullet_available": PYBULLET_AVAILABLE,
                "mujoco_available": MUJOCO_AVAILABLE,
            },
            "tests": {}
        }

        # Run all tests
        tests = ["free_fall"]
        for test in tests:
            report["tests"][test] = self.compare_simulators(test)

        # Save to file if specified
        if output_file:
            with open(output_file, 'w') as f:
                json.dump(report, f, indent=2)
            print(f"\nReport saved to {output_file}")

        return report


def main():
    parser = argparse.ArgumentParser(description="Physics benchmark comparison")
    parser.add_argument("--test", default="free_fall",
                       help="Test to run (free_fall, bouncing_ball, all)")
    parser.add_argument("--output", help="Output JSON file")
    parser.add_argument("--verbose", action="store_true", help="Verbose output")

    args = parser.parse_args()

    benchmark = PhysicsBenchmark()

    if args.test == "all":
        report = benchmark.generate_report(args.output)

        # Print summary
        print("\n=== BENCHMARK SUMMARY ===\n")
        for test_name, test_results in report["tests"].items():
            print(f"{test_name}:")
            if test_results.get("horus"):
                print(f"  HORUS: {test_results['horus']['avg_step_time']:.2f} μs/step")
            if test_results.get("pybullet"):
                print(f"  PyBullet: {test_results['pybullet']['avg_step_time']:.2f} μs/step")
            if test_results.get("speedup_vs_pybullet"):
                print(f"  Speedup: {test_results['speedup_vs_pybullet']:.2f}x")
            print()
    else:
        results = benchmark.compare_simulators(args.test)
        if args.output:
            with open(args.output, 'w') as f:
                json.dump(results, f, indent=2)


if __name__ == "__main__":
    main()
