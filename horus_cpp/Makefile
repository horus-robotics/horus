# Makefile for HORUS C++ API and examples
CXX = g++
CXXFLAGS = -std=c++17 -Wall -O2 -I./include
LDFLAGS = -L../target/release -lhorus_cpp -lpthread -ldl -lm

# Detect OS for rpath
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Linux)
    LDFLAGS += -Wl,-rpath,../target/release
endif
ifeq ($(UNAME_S),Darwin)
    LDFLAGS += -Wl,-rpath,@loader_path/../target/release
endif

# Examples
EXAMPLES = lidar_driver camera_driver robot_arm_driver

.PHONY: all clean build examples

all: build examples

build:
	cargo build --release

examples: $(EXAMPLES)

lidar_driver: build examples/lidar_driver.cpp
	$(CXX) $(CXXFLAGS) examples/lidar_driver.cpp -o lidar_driver $(LDFLAGS)

camera_driver: build examples/camera_driver.cpp
	$(CXX) $(CXXFLAGS) examples/camera_driver.cpp -o camera_driver $(LDFLAGS)

robot_arm_driver: build examples/robot_arm_driver.cpp
	$(CXX) $(CXXFLAGS) examples/robot_arm_driver.cpp -o robot_arm_driver $(LDFLAGS)

clean:
	rm -f $(EXAMPLES)
	cargo clean

# Run examples
run-lidar: lidar_driver
	./lidar_driver

run-camera: camera_driver
	./camera_driver

run-robot: robot_arm_driver
	./robot_arm_driver

help:
	@echo "HORUS C++ API Build"
	@echo "==================="
	@echo "make all       - Build library and examples"
	@echo "make build     - Build Rust library only"
	@echo "make examples  - Build C++ examples"
	@echo "make clean     - Clean all builds"
	@echo ""
	@echo "Run examples:"
	@echo "make run-lidar   - Run LiDAR driver"
	@echo "make run-camera  - Run camera driver"
	@echo "make run-robot   - Run robot arm driver"