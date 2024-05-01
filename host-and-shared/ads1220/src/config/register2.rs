use bitfield_struct::bitfield;

/// Voltage reference selection
/// These bits select the voltage reference source that is used for the conversion.
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Vref {
    /// 00 : Internal 2.048-V reference selected (default)
    Internal = 0b00,
    /// 01 : External reference selected using dedicated REFP0 and REFN0 inputs
    ExternalRefp0Refn0 = 0b01,
    /// 10 : External reference selected using AIN0/REFP1 and AIN3/REFN1 inputs
    ExternalRefp1Refn1 = 0b10,
    /// 11 : Analog supply (AVDD – AVSS) used as reference
    AnalogSupply = 0b11,
}

impl Default for Vref {
    fn default() -> Self {
        Self::Internal
    }
}

impl Vref {
    const fn into_bits(self) -> u8 {
        self as _
    }

    const fn from_bits(_value: u8) -> Self {
        unimplemented!()
    }
}

/// FIR filter configuration
/// These bits configure the filter coefficients for the internal FIR filter.
/// Only use these bits together with the 20-SPS setting in normal mode and the 5-
/// SPS setting in duty-cycle mode. Set to 00 for all other data rates.
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum FirFilter {
    /// 00 : No 50-Hz or 60-Hz rejection (default)
    NoRejection = 0b00,
    /// 01 : Simultaneous 50-Hz and 60-Hz rejection
    Reject5060 = 0b01,
    /// 10 : 50-Hz rejection only
    Reject50 = 0b10,
    /// 11 : 60-Hz rejection only
    Reject60 = 0b11,
}

impl Default for FirFilter {
    fn default() -> Self {
        Self::NoRejection
    }
}

impl FirFilter {
    const fn into_bits(self) -> u8 {
        self as _
    }

    const fn from_bits(_value: u8) -> Self {
        unimplemented!()
    }
}

/// Low-side power switch configuration
/// This bit configures the behavior of the low-side switch connected between
/// AIN3/REFN1 and AVSS.
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum LowSidePower {
    /// 0 : Switch is always open (default)
    AlwaysOpen = 0b0,
    /// 1 : Switch automatically closes when the START/SYNC command is sent and
    /// opens when the POWERDOWN command is issued
    ClosedWhenActive = 0b1,
}

impl Default for LowSidePower {
    fn default() -> Self {
        Self::AlwaysOpen
    }
}

impl LowSidePower {
    const fn into_bits(self) -> u8 {
        self as _
    }

    const fn from_bits(_value: u8) -> Self {
        unimplemented!()
    }
}

/// IDAC current setting
/// These bits set the current for both IDAC1 and IDAC2 excitation current sources.
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum IdacCurrent {
    /// 000 : Off (default)
    Off = 0b000,
    /// 001 : 10 µA
    Ua10 = 0b001,
    /// 010 : 50 µA
    Ua50 = 0b010,
    /// 011 : 100 µA
    Ua100 = 0b011,
    /// 100 : 250 µA
    Ua250 = 0b100,
    /// 101 : 500 µA
    Ua500 = 0b101,
    /// 110 : 1000 µA
    Ua1000 = 0b110,
    /// 111 : 1500 µA
    Ua1500 = 0b111,
}

impl Default for IdacCurrent {
    fn default() -> Self {
        Self::Off
    }
}

impl IdacCurrent {
    const fn into_bits(self) -> u8 {
        self as _
    }

    const fn from_bits(_value: u8) -> Self {
        unimplemented!()
    }
}

#[bitfield(u8, order = Msb)]
pub struct Register2 {
    #[bits(2, default=Vref::Internal)]
    pub vref: Vref,
    #[bits(2, default=FirFilter::NoRejection)]
    pub fir_filter: FirFilter,
    #[bits(1, default=LowSidePower::AlwaysOpen)]
    pub low_side_power: LowSidePower,
    #[bits(3, default=IdacCurrent::Off)]
    pub idac_current: IdacCurrent,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let reg = Register2::new();
        assert_eq!(u8::from(reg), 0u8, "default register value is a zero byte");
    }
}