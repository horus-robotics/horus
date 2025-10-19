/// checksum calculation for RPLIDAR protocol
pub struct Checksum {
    current: u8
}

impl Checksum {
    /// create a new `Checksum`
    pub fn new() -> Checksum {
        Checksum { current: 0 }
    }

    /*
    /// reset the calculation of `Checksum`
    pub fn reset(&mut self) {
        self.current = 0;
    }
    */

    /// push data into the `Checksum`
    pub fn push(&mut self, data: u8) {
        self.current ^= data;
    }

    /// push slice into the `Checksum`
    pub fn push_slice(&mut self, data: &[u8]) {
        for i in 0..data.len() {
            self.current ^= data[i];
        }
    }

    /// output the calculated checksum
    pub fn checksum(&self) -> u8 {
        self.current
    }
}
