use bitfield_struct::bitfield;

/// Input multiplexer configuration
/// These bits configure the input multiplexer.
/// For settings where AINN = AVSS, the PGA must be disabled (PGA_BYPASS = 1)
/// and only gains 1, 2, and 4 can be used.
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Mux {
    /// 0000 : AINP = AIN0, AINN = AIN1 (default)
    Ain0Ain1 = 0b0000,
    /// 0001 : AINP = AIN0, AINN = AIN2
    Ain0Ain2 = 0b0001,
    /// 0010 : AINP = AIN0, AINN = AIN3
    Ain0Ain3 = 0b0010,
    /// 0011 : AINP = AIN1, AINN = AIN2
    Ain1Ain2 = 0b0011,
    /// 0100 : AINP = AIN1, AINN = AIN3
    Ain1Ain3 = 0b0100,
    /// 0101 : AINP = AIN2, AINN = AIN3
    Ain2Ain3 = 0b0101,
    /// 0110 : AINP = AIN1, AINN = AIN0
    Ain1Ain0 = 0b0110,
    /// 0111 : AINP = AIN3, AINN = AIN2
    Ain3Ain2 = 0b0111,
    /// 1000 : AINP = AIN0, AINN = AVSS
    Ain0Avss = 0b1000,
    /// 1001 : AINP = AIN1, AINN = AVSS
    Ain1Avss = 0b1001,
    /// 1010 : AINP = AIN2, AINN = AVSS
    Ain2Avss = 0b1010,
    /// 1011 : AINP = AIN3, AINN = AVSS
    Ain3Avss = 0b1011,
    /// 1100 : (V(REFPx) – V(REFNx)) / 4 monitor (PGA bypassed)
    VrefpVrefnMonitor = 0b1100,
    /// 1101 : (AVDD – AVSS) / 4 monitor (PGA bypassed)
    AvddAvssMonitor = 0b1101,
    /// 1110 : AINP and AINN shorted to (AVDD + AVSS) / 2
    AinpAinnShorted = 0b1110,
    /// 1111 : Reserved
    Reserved = 0b1111,
}

impl Default for Mux {
    fn default() -> Self {
        Self::Ain0Ain1
    }
}

impl Mux {
    const fn into_bits(self) -> u8 {
        self as _
    }

    const fn from_bits(_value: u8) -> Self {
        unimplemented!()
    }
}

/// Gain configuration
/// These bits configure the device gain.
/// Gains 1, 2, and 4 can be used without the PGA. In this case, gain is obtained by
/// a switched-capacitor structure.
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Gain {
    // can be used with and without PGA
    /// default
    Gain1 = 0b000,
    Gain2 = 0b001,
    Gain4 = 0b010,
    // can't be used without PGA
    Gain8 = 0b011,
    Gain16 = 0b100,
    Gain32 = 0b101,
    Gain64 = 0b110,
    Gain128 = 0b111,
}

impl Default for Gain {
    fn default() -> Self {
        Self::Gain1
    }
}

impl Gain {
    const fn into_bits(self) -> u8 {
        self as _
    }

    const fn from_bits(_value: u8) -> Self {
        unimplemented!()
    }
}

/// Disables and bypasses the internal low-noise PGA
/// Disabling the PGA reduces overall power consumption and allows the commonmode voltage range (VCM) to span from AVSS – 0.1 V to AVDD + 0.1 V.
/// The PGA can only be disabled for gains 1, 2, and 4.
/// The PGA is always enabled for gain settings 8 to 128, regardless of the
/// PGA_BYPASS setting.
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Pga {
    /// 0: PGA is enabled (default)
    Enabled = 0b0,
    /// 1: PGA disabled and bypassed
    Bypassed = 0b1,
}

impl Default for Pga {
    fn default() -> Self {
        Self::Enabled
    }
}

impl Pga {
    const fn into_bits(self) -> u8 {
        self as _
    }

    const fn from_bits(_value: u8) -> Self {
        unimplemented!()
    }
}

#[bitfield(u8, order = Msb)]
pub struct Register0 {
    #[bits(4, default=Mux::Ain0Ain1)]
    pub mux: Mux,
    #[bits(3, default=Gain::Gain1)]
    pub gain: Gain,
    #[bits(1, default=Pga::Enabled)]
    pub pga: Pga,
}

// TODO: make tests work
// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn test_register0() {
//         let reg = Register0::new();
//         assert_eq!(u8::from(reg), 0u8, "default register value is a zero byte");
//
//         let reg = Register0::new()
//             .with_mux(Mux::AvddAvssMonitor)
//             .with_gain(Gain::Gain64)
//             .with_pga(Pga::Bypassed);
//         assert_eq!(u8::from(reg), 0b1101_110_1u8);
//
//         let reg = Register0::new().with_mux(Mux::Ain0Ain2);
//         assert_eq!(u8::from(reg), 0b0001_000_0u8);
//     }
// }
