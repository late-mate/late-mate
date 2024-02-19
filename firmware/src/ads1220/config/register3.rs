use crate::ads1220::config::register2::{FirFilter, LowSidePower};
use bitfield_struct::bitfield;

/// IDAC1 routing configuration
/// These bits select the channel where IDAC1 is routed to.
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Idac1Routing {
    /// 000 : IDAC1 disabled (default)
    Disabled = 0b000,
    /// 001 : IDAC1 connected to AIN0/REFP1
    Ain0 = 0b001,
    /// 010 : IDAC1 connected to AIN1
    Ain1 = 0b010,
    /// 011 : IDAC1 connected to AIN2
    Ain2 = 0b011,
    /// 100 : IDAC1 connected to AIN3/REFN1
    Ain3 = 0b100,
    /// 101 : IDAC1 connected to REFP0
    Refp0 = 0b101,
    /// 110 : IDAC1 connected to REFN0
    Refn0 = 0b110,
    /// 111 : Reserved
    Reserved = 0b111,
}

impl Default for Idac1Routing {
    fn default() -> Self {
        Self::Disabled
    }
}

impl Idac1Routing {
    const fn into_bits(self) -> u8 {
        self as _
    }

    const fn from_bits(_value: u8) -> Self {
        unimplemented!()
    }
}

/// IDAC2 routing configuration
/// These bits select the channel where IDAC2 is routed to.
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Idac2Routing {
    /// 000 : IDAC2 disabled (default)
    Disabled = 0b000,
    /// 001 : IDAC2 connected to AIN0/REFP1
    Ain0 = 0b001,
    /// 010 : IDAC2 connected to AIN1
    Ain1 = 0b010,
    /// 011 : IDAC2 connected to AIN2
    Ain2 = 0b011,
    /// 100 : IDAC2 connected to AIN3/REFN1
    Ain3 = 0b100,
    /// 101 : IDAC2 connected to REFP0
    Refp0 = 0b101,
    /// 110 : IDAC2 connected to REFN0
    Refn0 = 0b110,
    /// 111 : Reserved
    Reserved = 0b111,
}

impl Default for Idac2Routing {
    fn default() -> Self {
        Self::Disabled
    }
}

impl Idac2Routing {
    const fn into_bits(self) -> u8 {
        self as _
    }

    const fn from_bits(_value: u8) -> Self {
        unimplemented!()
    }
}

/// DRDY mode
/// This bit controls the behavior of the DOUT/nDRDY pin when new data are ready.
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum DrdyMode {
    /// 0 : Only the dedicated nDRDY pin is used to indicate when data are ready (default)
    DrdyOnly = 0b0,
    /// 1 : Data ready is indicated simultaneously on DOUT/nDRDY and nDRDY
    DoutDrdy = 0b1,
}

impl Default for DrdyMode {
    fn default() -> Self {
        Self::DrdyOnly
    }
}

impl DrdyMode {
    const fn into_bits(self) -> u8 {
        self as _
    }

    const fn from_bits(_value: u8) -> Self {
        unimplemented!()
    }
}

#[bitfield(u8, order = Msb)]
pub struct Register3 {
    #[bits(3, default=Idac1Routing::Disabled)]
    pub idac1_routing: Idac1Routing,
    #[bits(3, default=Idac2Routing::Disabled)]
    pub idac2_routing: Idac2Routing,
    #[bits(1, default=DrdyMode::DrdyOnly)]
    pub drdy_mode: DrdyMode,
    /// reserved value, must always be 0
    #[bits(1)]
    __: u8,
}
