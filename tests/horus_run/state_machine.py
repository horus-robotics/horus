#!/usr/bin/env python3

class RobotState:
    IDLE = "idle"
    MOVING = "moving"
    STOPPED = "stopped"

def main():
    state = RobotState.IDLE
    print(f"Initial state: {state}")

    transitions = [RobotState.MOVING, RobotState.STOPPED, RobotState.IDLE]

    for new_state in transitions:
        state = new_state
        print(f"Transitioned to: {state}")

    print("State machine completed")
    return 0

if __name__ == "__main__":
    exit(main())
