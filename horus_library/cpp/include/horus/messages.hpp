// HORUS Message Library - C++ API
// Single include for all HORUS message types
//
// This header provides C++ message definitions that are binary-compatible
// with the Rust message definitions in horus_library/messages/

#ifndef HORUS_MESSAGES_HPP
#define HORUS_MESSAGES_HPP

// Geometry messages
#include "messages/geometry.hpp"

// Sensor messages
#include "messages/sensor.hpp"

// Vision messages
#include "messages/vision.hpp"

// Perception messages
#include "messages/perception.hpp"

// Navigation messages
#include "messages/navigation.hpp"

// Control messages
#include "messages/control.hpp"

// Diagnostics messages
#include "messages/diagnostics.hpp"

// Re-export all types into horus::messages namespace
namespace horus {
namespace messages {
    // All types are already in horus::messages namespace
    // This file just provides a single include point
}
}

#endif // HORUS_MESSAGES_HPP
