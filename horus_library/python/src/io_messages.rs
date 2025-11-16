// Python wrappers for I/O messages
use horus_library::messages::io;
use pyo3::prelude::*;

/// Python wrapper for DigitalIO
#[pyclass(module = "horus.library._library", name = "DigitalIO")]
#[derive(Clone)]
pub struct PyDigitalIO {
    pub(crate) inner: io::DigitalIO,
}

#[pymethods]
impl PyDigitalIO {
    #[new]
    #[pyo3(signature = (pin_count=8))]
    fn new(pin_count: u8) -> Self {
        Self {
            inner: io::DigitalIO::new(pin_count),
        }
    }

    #[getter]
    fn pin_count(&self) -> u8 {
        self.inner.pin_count
    }

    fn set_pin(&mut self, pin: u8, state: bool) -> bool {
        self.inner.set_pin(pin, state)
    }

    fn get_pin(&self, pin: u8) -> Option<bool> {
        self.inner.get_pin(pin)
    }

    fn set_pin_direction(&mut self, pin: u8, is_output: bool) -> bool {
        self.inner.set_pin_direction(pin, is_output)
    }

    fn set_pin_label(&mut self, pin: u8, label: &str) -> bool {
        self.inner.set_pin_label(pin, label)
    }

    fn get_pin_label(&self, pin: u8) -> Option<String> {
        self.inner.get_pin_label(pin)
    }

    fn count_active(&self) -> u8 {
        self.inner.count_active()
    }

    fn as_bitmask(&self) -> u32 {
        self.inner.as_bitmask()
    }

    fn from_bitmask(&mut self, mask: u32) {
        self.inner.from_bitmask(mask)
    }

    fn __repr__(&self) -> String {
        format!(
            "DigitalIO(pins={}, active={}, mask=0x{:X})",
            self.inner.pin_count,
            self.inner.count_active(),
            self.inner.as_bitmask()
        )
    }
}

/// Python wrapper for AnalogIO
#[pyclass(module = "horus.library._library", name = "AnalogIO")]
#[derive(Clone)]
pub struct PyAnalogIO {
    pub(crate) inner: io::AnalogIO,
}

#[pymethods]
impl PyAnalogIO {
    #[new]
    #[pyo3(signature = (channel_count=4))]
    fn new(channel_count: u8) -> Self {
        Self {
            inner: io::AnalogIO::new(channel_count),
        }
    }

    #[getter]
    fn channel_count(&self) -> u8 {
        self.inner.channel_count
    }

    #[getter]
    fn resolution_bits(&self) -> u8 {
        self.inner.resolution_bits
    }

    #[setter]
    fn set_resolution_bits(&mut self, value: u8) {
        self.inner.resolution_bits = value;
    }

    #[getter]
    fn sampling_frequency(&self) -> f32 {
        self.inner.sampling_frequency
    }

    #[setter]
    fn set_sampling_frequency(&mut self, value: f32) {
        self.inner.sampling_frequency = value;
    }

    fn set_channel(&mut self, channel: u8, value: f64) -> bool {
        self.inner.set_channel(channel, value)
    }

    fn get_channel(&self, channel: u8) -> Option<f64> {
        self.inner.get_channel(channel)
    }

    fn set_channel_range(&mut self, channel: u8, min: f64, max: f64) -> bool {
        self.inner.set_channel_range(channel, min, max)
    }

    fn set_channel_info(&mut self, channel: u8, label: &str, unit: &str) -> bool {
        self.inner.set_channel_info(channel, label, unit)
    }

    fn raw_to_engineering(&self, channel: u8, raw_value: u16) -> Option<f64> {
        self.inner.raw_to_engineering(channel, raw_value)
    }

    fn engineering_to_raw(&self, channel: u8, eng_value: f64) -> Option<u16> {
        self.inner.engineering_to_raw(channel, eng_value)
    }

    fn __repr__(&self) -> String {
        format!(
            "AnalogIO(channels={}, resolution={}bit, fs={:.0}Hz)",
            self.inner.channel_count, self.inner.resolution_bits, self.inner.sampling_frequency
        )
    }
}
