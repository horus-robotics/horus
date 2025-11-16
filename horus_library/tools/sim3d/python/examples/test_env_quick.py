#!/usr/bin/env python3
"""
Quick test to verify sim3d Python bindings work
"""

def test_import():
    """Test that sim3d_rl can be imported"""
    try:
        import sim3d_rl
        print("✓ sim3d_rl module imported successfully")
        return True
    except ImportError as e:
        print(f"✗ Failed to import sim3d_rl: {e}")
        print("\nTo fix: run 'maturin develop --release --features python' in sim3d directory")
        return False

def test_environment_creation():
    """Test creating a simple environment"""
    try:
        import sim3d_rl
        
        print("\nTesting environment creation...")
        env = sim3d_rl.make_env("reaching", obs_dim=10, action_dim=6)
        print("✓ Environment created successfully")
        return env
    except Exception as e:
        print(f"✗ Failed to create environment: {e}")
        return None

def test_reset(env):
    """Test environment reset"""
    if env is None:
        return False
    
    try:
        print("\nTesting reset()...")
        obs = env.reset()
        print(f"✓ Reset successful, observation shape: {obs.shape}")
        print(f"  Observation: {obs}")
        return True
    except Exception as e:
        print(f"✗ Reset failed: {e}")
        return False

def test_step(env):
    """Test environment step"""
    if env is None:
        return False
    
    try:
        print("\nTesting step()...")
        import numpy as np
        
        # Random action
        action = np.random.randn(6).astype(np.float32)
        obs, reward, done, truncated, info = env.step(action)
        
        print(f"✓ Step successful")
        print(f"  Observation shape: {obs.shape}")
        print(f"  Reward: {reward:.3f}")
        print(f"  Done: {done}")
        print(f"  Truncated: {truncated}")
        print(f"  Info: {info}")
        return True
    except Exception as e:
        print(f"✗ Step failed: {e}")
        return False

def test_episode(env):
    """Run a short episode"""
    if env is None:
        return False
    
    try:
        print("\nRunning 10-step episode...")
        import numpy as np
        
        obs = env.reset()
        total_reward = 0
        
        for step in range(10):
            action = np.random.randn(6).astype(np.float32)
            obs, reward, done, truncated, info = env.step(action)
            total_reward += reward
            
            if done or truncated:
                print(f"  Episode ended at step {step + 1}")
                break
        
        print(f"✓ Episode completed")
        print(f"  Total reward: {total_reward:.3f}")
        print(f"  Steps: {step + 1}")
        return True
    except Exception as e:
        print(f"✗ Episode failed: {e}")
        return False

def main():
    """Run all tests"""
    print("=" * 60)
    print("sim3d Python Bindings Quick Test")
    print("=" * 60)
    
    # Test import
    if not test_import():
        return False
    
    # Create environment
    env = test_environment_creation()
    if env is None:
        return False
    
    # Test reset
    if not test_reset(env):
        return False
    
    # Test step
    if not test_step(env):
        return False
    
    # Test episode
    if not test_episode(env):
        return False
    
    print("\n" + "=" * 60)
    print("✓ ALL TESTS PASSED!")
    print("=" * 60)
    print("\nsim3d Python bindings are working correctly!")
    print("You can now:")
    print("  - Run full training: python examples/train_ppo.py")
    print("  - Benchmark performance: python examples/benchmark_env.py")
    
    return True

if __name__ == "__main__":
    import sys
    success = main()
    sys.exit(0 if success else 1)
