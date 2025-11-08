// Hardware Abstraction Layer for RTOS integration

use crate::error::HorusResult;
use std::time::Duration;

/// Platform information
#[derive(Debug, Clone)]
pub struct PlatformInfo {
    pub processor: ProcessorInfo,
    pub memory: MemoryInfo,
    pub peripherals: Vec<Peripheral>,
    pub board_name: String,
    pub vendor: String,
}

/// Processor information
#[derive(Debug, Clone)]
pub struct ProcessorInfo {
    pub arch: ProcessorArch,
    pub cores: u32,
    pub frequency_hz: u64,
    pub fpu: bool,
    pub mpu: bool, // Memory Protection Unit
    pub cache_l1: usize,
    pub cache_l2: usize,
    pub vendor_id: String,
}

/// Processor architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessorArch {
    ARM32,
    ARM64,
    X86,
    X86_64,
    RISCV32,
    RISCV64,
    PowerPC,
    MIPS,
}

/// Memory information
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    pub ram_size: usize,
    pub flash_size: usize,
    pub eeprom_size: usize,
    pub sram_regions: Vec<MemoryRegion>,
    pub dma_capable: bool,
}

/// Memory region descriptor
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub name: String,
    pub base_addr: usize,
    pub size: usize,
    pub cacheable: bool,
    pub executable: bool,
    pub dma_capable: bool,
}

/// Peripheral device
#[derive(Debug, Clone)]
pub struct Peripheral {
    pub name: String,
    pub peripheral_type: PeripheralType,
    pub base_addr: usize,
    pub irq_number: Option<u32>,
}

/// Peripheral types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeripheralType {
    UART,
    SPI,
    I2C,
    CAN,
    Ethernet,
    USB,
    GPIO,
    Timer,
    PWM,
    ADC,
    DAC,
    DMA,
    Watchdog,
}

/// Hardware timer for precise timing
pub trait HardwareTimer {
    /// Initialize timer
    fn init(&mut self, frequency: u32) -> HorusResult<()>;

    /// Start timer
    fn start(&mut self) -> HorusResult<()>;

    /// Stop timer
    fn stop(&mut self) -> HorusResult<()>;

    /// Get current counter value
    fn get_counter(&self) -> u64;

    /// Set compare value for interrupt generation
    fn set_compare(&mut self, value: u64) -> HorusResult<()>;

    /// Enable timer interrupt
    fn enable_interrupt(&mut self) -> HorusResult<()>;

    /// Disable timer interrupt
    fn disable_interrupt(&mut self) -> HorusResult<()>;

    /// Get timer frequency in Hz
    fn get_frequency(&self) -> u32;

    /// Convert ticks to microseconds
    fn ticks_to_us(&self, ticks: u64) -> u64 {
        (ticks * 1_000_000) / self.get_frequency() as u64
    }

    /// Convert microseconds to ticks
    fn us_to_ticks(&self, us: u64) -> u64 {
        (us * self.get_frequency() as u64) / 1_000_000
    }
}

/// GPIO pin abstraction
pub trait GPIOPin {
    /// Set pin mode
    fn set_mode(&mut self, mode: PinMode) -> HorusResult<()>;

    /// Read pin value
    fn read(&self) -> bool;

    /// Write pin value
    fn write(&mut self, value: bool) -> HorusResult<()>;

    /// Toggle pin value
    fn toggle(&mut self) -> HorusResult<()>;

    /// Enable interrupt on pin
    fn enable_interrupt(&mut self, trigger: InterruptTrigger) -> HorusResult<()>;

    /// Disable interrupt on pin
    fn disable_interrupt(&mut self) -> HorusResult<()>;
}

/// GPIO pin mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinMode {
    Input,
    Output,
    InputPullUp,
    InputPullDown,
    OutputOpenDrain,
    Alternate(u8), // Alternate function number
    Analog,
}

/// Interrupt trigger type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptTrigger {
    RisingEdge,
    FallingEdge,
    BothEdges,
    LowLevel,
    HighLevel,
}

/// UART interface
pub trait UARTInterface {
    /// Initialize UART with baud rate
    fn init(&mut self, baud_rate: u32) -> HorusResult<()>;

    /// Send data
    fn send(&mut self, data: &[u8]) -> HorusResult<()>;

    /// Receive data
    fn receive(&mut self, buffer: &mut [u8]) -> HorusResult<usize>;

    /// Send single byte
    fn send_byte(&mut self, byte: u8) -> HorusResult<()>;

    /// Receive single byte
    fn receive_byte(&mut self) -> HorusResult<u8>;

    /// Check if data available
    fn available(&self) -> bool;

    /// Flush transmit buffer
    fn flush(&mut self) -> HorusResult<()>;

    /// Set flow control
    fn set_flow_control(&mut self, flow: FlowControl) -> HorusResult<()>;
}

/// UART flow control
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowControl {
    None,
    Hardware,
    Software,
}

/// SPI interface
pub trait SPIInterface {
    /// Initialize SPI
    fn init(&mut self, config: SPIConfig) -> HorusResult<()>;

    /// Transfer data (full duplex)
    fn transfer(&mut self, send: &[u8], receive: &mut [u8]) -> HorusResult<()>;

    /// Send data only
    fn send(&mut self, data: &[u8]) -> HorusResult<()>;

    /// Receive data only
    fn receive(&mut self, buffer: &mut [u8]) -> HorusResult<()>;

    /// Set chip select
    fn set_cs(&mut self, active: bool) -> HorusResult<()>;
}

/// SPI configuration
#[derive(Debug, Clone, Copy)]
pub struct SPIConfig {
    pub frequency: u32,
    pub mode: SPIMode,
    pub bit_order: BitOrder,
    pub data_bits: u8,
}

/// SPI mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SPIMode {
    Mode0, // CPOL=0, CPHA=0
    Mode1, // CPOL=0, CPHA=1
    Mode2, // CPOL=1, CPHA=0
    Mode3, // CPOL=1, CPHA=1
}

/// Bit order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitOrder {
    MSBFirst,
    LSBFirst,
}

/// I2C interface
pub trait I2CInterface {
    /// Initialize I2C
    fn init(&mut self, frequency: u32) -> HorusResult<()>;

    /// Write data to device
    fn write(&mut self, addr: u8, data: &[u8]) -> HorusResult<()>;

    /// Read data from device
    fn read(&mut self, addr: u8, buffer: &mut [u8]) -> HorusResult<()>;

    /// Write then read (common pattern for register access)
    fn write_read(&mut self, addr: u8, write: &[u8], read: &mut [u8]) -> HorusResult<()>;

    /// Scan for devices
    fn scan(&mut self) -> HorusResult<Vec<u8>>;
}

/// CAN interface
pub trait CANInterface {
    /// Initialize CAN
    fn init(&mut self, baud_rate: u32) -> HorusResult<()>;

    /// Send CAN frame
    fn send(&mut self, frame: &CANFrame) -> HorusResult<()>;

    /// Receive CAN frame
    fn receive(&mut self) -> HorusResult<CANFrame>;

    /// Set acceptance filter
    fn set_filter(&mut self, filter: CANFilter) -> HorusResult<()>;

    /// Check if frame available
    fn available(&self) -> bool;
}

/// CAN frame
#[derive(Debug, Clone)]
pub struct CANFrame {
    pub id: u32,
    pub extended: bool,
    pub rtr: bool, // Remote Transmission Request
    pub data: Vec<u8>,
}

/// CAN filter
#[derive(Debug, Clone, Copy)]
pub struct CANFilter {
    pub id: u32,
    pub mask: u32,
    pub extended: bool,
}

/// PWM interface
pub trait PWMInterface {
    /// Initialize PWM
    fn init(&mut self, frequency: u32) -> HorusResult<()>;

    /// Set duty cycle (0.0 to 1.0)
    fn set_duty(&mut self, duty: f32) -> HorusResult<()>;

    /// Set frequency
    fn set_frequency(&mut self, frequency: u32) -> HorusResult<()>;

    /// Enable output
    fn enable(&mut self) -> HorusResult<()>;

    /// Disable output
    fn disable(&mut self) -> HorusResult<()>;
}

/// ADC interface
pub trait ADCInterface {
    /// Initialize ADC
    fn init(&mut self, resolution: u8) -> HorusResult<()>;

    /// Read single channel
    fn read(&mut self, channel: u8) -> HorusResult<u16>;

    /// Read multiple channels
    fn read_multiple(&mut self, channels: &[u8], buffer: &mut [u16]) -> HorusResult<()>;

    /// Start continuous conversion
    fn start_continuous(&mut self, channel: u8) -> HorusResult<()>;

    /// Stop continuous conversion
    fn stop_continuous(&mut self) -> HorusResult<()>;

    /// Get reference voltage in millivolts
    fn get_vref(&self) -> u32;

    /// Convert raw value to voltage
    fn to_voltage(&self, raw: u16) -> f32 {
        (raw as f32 / ((1 << self.get_resolution()) - 1) as f32) * (self.get_vref() as f32 / 1000.0)
    }

    /// Get ADC resolution in bits
    fn get_resolution(&self) -> u8;
}

/// DMA controller interface
pub trait DMAController {
    /// Configure DMA channel
    fn configure(&mut self, channel: u8, config: DMAConfig) -> HorusResult<()>;

    /// Start DMA transfer
    fn start(&mut self, channel: u8) -> HorusResult<()>;

    /// Stop DMA transfer
    fn stop(&mut self, channel: u8) -> HorusResult<()>;

    /// Check if transfer complete
    fn is_complete(&self, channel: u8) -> bool;

    /// Get remaining transfer count
    fn get_remaining(&self, channel: u8) -> usize;

    /// Set transfer complete callback
    fn set_callback(&mut self, channel: u8, callback: fn()) -> HorusResult<()>;
}

/// DMA configuration
#[derive(Debug, Clone)]
pub struct DMAConfig {
    pub source: usize,
    pub destination: usize,
    pub count: usize,
    pub transfer_type: DMATransferType,
    pub data_width: DMADataWidth,
    pub priority: DMAPriority,
    pub circular: bool,
}

/// DMA transfer type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DMATransferType {
    MemoryToMemory,
    PeripheralToMemory,
    MemoryToPeripheral,
}

/// DMA data width
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DMADataWidth {
    Byte,
    HalfWord,
    Word,
}

/// DMA priority
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DMAPriority {
    Low,
    Medium,
    High,
    VeryHigh,
}

/// Watchdog timer interface
pub trait WatchdogTimer {
    /// Initialize watchdog
    fn init(&mut self, timeout: Duration) -> HorusResult<()>;

    /// Start watchdog
    fn start(&mut self) -> HorusResult<()>;

    /// Stop watchdog
    fn stop(&mut self) -> HorusResult<()>;

    /// Feed/reset watchdog
    fn feed(&mut self) -> HorusResult<()>;

    /// Set timeout
    fn set_timeout(&mut self, timeout: Duration) -> HorusResult<()>;

    /// Enable early warning interrupt
    fn enable_warning(&mut self, before: Duration) -> HorusResult<()>;
}

/// Main Hardware Abstraction Layer
pub trait HardwareAbstractionLayer {
    /// Get platform information
    fn platform_info(&self) -> PlatformInfo;

    /// Initialize hardware
    fn init(&mut self) -> HorusResult<()>;

    /// Get hardware timer
    fn get_timer(&self, id: u8) -> Option<Box<dyn HardwareTimer>>;

    /// Get GPIO pin
    fn get_gpio(&self, port: u8, pin: u8) -> Option<Box<dyn GPIOPin>>;

    /// Get UART interface
    fn get_uart(&self, id: u8) -> Option<Box<dyn UARTInterface>>;

    /// Get SPI interface
    fn get_spi(&self, id: u8) -> Option<Box<dyn SPIInterface>>;

    /// Get I2C interface
    fn get_i2c(&self, id: u8) -> Option<Box<dyn I2CInterface>>;

    /// Get CAN interface
    fn get_can(&self, id: u8) -> Option<Box<dyn CANInterface>>;

    /// Get PWM interface
    fn get_pwm(&self, id: u8) -> Option<Box<dyn PWMInterface>>;

    /// Get ADC interface
    fn get_adc(&self, id: u8) -> Option<Box<dyn ADCInterface>>;

    /// Get DMA controller
    fn get_dma(&self) -> Option<Box<dyn DMAController>>;

    /// Get watchdog timer
    fn get_watchdog(&self) -> Option<Box<dyn WatchdogTimer>>;

    /// Delay for microseconds
    fn delay_us(&self, us: u32);

    /// Delay for milliseconds
    fn delay_ms(&self, ms: u32) {
        self.delay_us(ms * 1000);
    }

    /// Get system clock in Hz
    fn get_system_clock(&self) -> u32;

    /// Get peripheral clock in Hz
    fn get_peripheral_clock(&self, peripheral: PeripheralType) -> u32;

    /// Enable peripheral clock
    fn enable_peripheral(&mut self, peripheral: PeripheralType) -> HorusResult<()>;

    /// Disable peripheral clock
    fn disable_peripheral(&mut self, peripheral: PeripheralType) -> HorusResult<()>;

    /// System reset
    fn system_reset(&mut self) -> !;

    /// Enter low power mode
    fn enter_low_power(&mut self) -> HorusResult<()>;

    /// Exit low power mode
    fn exit_low_power(&mut self) -> HorusResult<()>;

    /// Get unique device ID
    fn get_device_id(&self) -> [u8; 12];

    /// Get random number
    fn get_random(&mut self) -> u32;
}
