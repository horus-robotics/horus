# Release Note for Slamtec RPLIDAR Public SDK for Rust v0.5.0

* new feature: rplidar_drv::sort_scan (ascendScan in C++)
* reform: renamed rplidar_drv::protocol::RplidarProtocol to RplidarHostProtocol
* improve: eliminated dependency to libudev to avoid cross compile failure on linux
