use bitfield_struct::bitfield;

/// Data rate
/// These bits control the data rate setting depending on the selected operating
/// mode.
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum DataRate {
    /// 20 SPS in Normal mode, 5 SPS in Duty-Cycle mode, 40 SPS in Turbo mode (default)
    Normal20 = 0b000,
    /// 45 SPS in Normal mode, 11.25 SPS in Duty-Cycle mode, 90 SPS in Turbo mode
    Normal45 = 0b001,
    /// 90 SPS in Normal mode, 22.5 SPS in Duty-Cycle mode, 180 SPS in Turbo mode
    Normal90 = 0b010,
    /// 175 SPS in Normal mode, 44 SPS in Duty-Cycle mode, 350 SPS in Turbo mode
    Normal175 = 0b011,
    /// 330 SPS in Normal mode, 82.5 SPS in Duty-Cycle mode, 660 SPS in Turbo mode
    Normal330 = 0b100,
    /// 600 SPS in Normal mode, 150 SPS in Duty-Cycle mode, 1200 SPS in Turbo mode
    Normal600 = 0b101,
    /// 1000 SPS in Normal mode, 250 SPS in Duty-Cycle mode, 2000 SPS in Turbo mode
    Normal1000 = 0b110,
    /// Reserved
    Reserved = 0b111,
}

impl Default for DataRate {
    fn default() -> Self {
        Self::Normal20
    }
}

impl DataRate {
    const fn into_bits(self) -> u8 {
        self as _
    }

    const fn from_bits(_value: u8) -> Self {
        unimplemented!()
    }
}

/// Operating mode
/// These bits control the operating mode the device operates in.
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Mode {
    /// 00 : Normal mode (256-kHz modulator clock, default)
    Normal = 0b00,
    /// 01 : Duty-cycle mode (internal duty cycle of 1:4)
    DutyCycle = 0b01,
    /// 10 : Turbo mode (512-kHz modulator clock)
    Turbo = 0b10,
    /// 11 : Reserved
    Reserved = 0b11,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Normal
    }
}

impl Mode {
    const fn into_bits(self) -> u8 {
        self as _
    }

    const fn from_bits(_value: u8) -> Self {
        unimplemented!()
    }
}

/// Conversion mode
/// This bit sets the conversion mode for the device.
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum ConversionMode {
    /// 0 : Single-shot mode (default)
    SingleShot = 0b0,
    /// 1 : Continuous conversion mode
    Continuous = 0b1,
}

impl Default for ConversionMode {
    fn default() -> Self {
        Self::SingleShot
    }
}

impl ConversionMode {
    const fn into_bits(self) -> u8 {
        self as _
    }

    const fn from_bits(_value: u8) -> Self {
        unimplemented!()
    }
}

/// Temperature sensor mode
/// This bit enables the internal temperature sensor and puts the device in
/// temperature sensor mode.
/// The settings of configuration register 0 have no effect and the device uses the
/// internal reference for measurement when temperature sensor mode is enabled.
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum TempSensor {
    /// 0 : Disables temperature sensor (default)
    Disabled = 0b0,
    /// 1 : Enables temperature sensor
    Enabled = 0b1,
}

impl Default for TempSensor {
    fn default() -> Self {
        Self::Disabled
    }
}

impl TempSensor {
    const fn into_bits(self) -> u8 {
        self as _
    }

    const fn from_bits(_value: u8) -> Self {
        unimplemented!()
    }
}

/// Burn-out current source
/// This bit controls the 10-µA, burn-out current sources.
/// The burn-out current sources can be used to detect sensor faults such as wire
/// breaks and shorted sensors.
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Bcs {
    /// 0 : Current sources off (default)
    Disabled = 0b0,
    /// 1 : Current sources on
    Enabled = 0b1,
}

impl Default for Bcs {
    fn default() -> Self {
        Self::Disabled
    }
}

impl Bcs {
    const fn into_bits(self) -> u8 {
        self as _
    }

    const fn from_bits(_value: u8) -> Self {
        unimplemented!()
    }
}

#[bitfield(u8, order = Msb)]
pub struct Register1 {
    #[bits(3, default=DataRate::Normal20)]
    pub data_rate: DataRate,
    #[bits(2, default=Mode::Normal)]
    pub mode: Mode,
    #[bits(1, default=ConversionMode::SingleShot)]
    pub conversion_mode: ConversionMode,
    #[bits(1, default=TempSensor::Disabled)]
    pub temp_sensor: TempSensor,
    #[bits(1, default=Bcs::Disabled)]
    pub bcs: Bcs,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let reg = Register1::new();
        assert_eq!(u8::from(reg), 0u8, "default register value is a zero byte");
    }
}