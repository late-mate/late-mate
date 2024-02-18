use bilge::prelude::*;

/// Input multiplexer configuration
/// These bits configure the input multiplexer.
/// For settings where AINN = AVSS, the PGA must be disabled (PGA_BYPASS = 1)
/// and only gains 1, 2, and 4 can be used.
#[bitsize(4)]
#[derive(FromBits)]
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

/// Gain configuration
/// These bits configure the device gain.
/// Gains 1, 2, and 4 can be used without the PGA. In this case, gain is obtained by
/// a switched-capacitor structure.
#[bitsize(3)]
#[derive(FromBits)]
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

/// Disables and bypasses the internal low-noise PGA
/// Disabling the PGA reduces overall power consumption and allows the commonmode voltage range (VCM) to span from AVSS – 0.1 V to AVDD + 0.1 V.
/// The PGA can only be disabled for gains 1, 2, and 4.
/// The PGA is always enabled for gain settings 8 to 128, regardless of the
/// PGA_BYPASS setting.
#[bitsize(1)]
#[derive(FromBits)]
pub enum PgaBypass {
    /// 0: PGA is enabled (default)
    Enabled = 0b0,
    /// 1: PGA disabled and bypassed
    Bypassed = 0b1,
}

impl Default for PgaBypass {
    fn default() -> Self {
        Self::Enabled
    }
}

#[bitsize(8)]
#[derive(FromBits)]
pub struct Register0 {
    pga_bypass: PgaBypass,
    gain: Gain,
    mux: Mux,
}

impl Register0 {
    pub fn to_value(self) -> u8 {
        self.value
    }
}

/// Data rate
/// These bits control the data rate setting depending on the selected operating
/// mode.
#[bitsize(3)]
#[derive(FromBits)]
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

/// Operating mode
/// These bits control the operating mode the device operates in.
#[bitsize(2)]
#[derive(FromBits)]
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

/// Conversion mode
/// This bit sets the conversion mode for the device.
#[bitsize(1)]
#[derive(FromBits)]
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

/// Temperature sensor mode
/// This bit enables the internal temperature sensor and puts the device in
/// temperature sensor mode.
/// The settings of configuration register 0 have no effect and the device uses the
/// internal reference for measurement when temperature sensor mode is enabled.
#[bitsize(1)]
#[derive(FromBits)]
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

/// Burn-out current source
/// This bit controls the 10-µA, burn-out current sources.
/// The burn-out current sources can be used to detect sensor faults such as wire
/// breaks and shorted sensors.
#[bitsize(1)]
#[derive(FromBits)]
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

#[bitsize(8)]
pub struct Register1 {
    data_rate: DataRate,
    mode: Mode,
    conversion_mode: ConversionMode,
    temp_sensor: TempSensor,
    bcs: Bcs,
}

/// Voltage reference selection
/// These bits select the voltage reference source that is used for the conversion.
#[bitsize(2)]
#[derive(FromBits)]
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

/// FIR filter configuration
/// These bits configure the filter coefficients for the internal FIR filter.
/// Only use these bits together with the 20-SPS setting in normal mode and the 5-
/// SPS setting in duty-cycle mode. Set to 00 for all other data rates.
#[bitsize(2)]
#[derive(FromBits)]
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

/// Low-side power switch configuration
/// This bit configures the behavior of the low-side switch connected between
/// AIN3/REFN1 and AVSS.
#[bitsize(1)]
#[derive(FromBits)]
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

/// IDAC current setting
/// These bits set the current for both IDAC1 and IDAC2 excitation current sources.
#[bitsize(3)]
#[derive(FromBits)]
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

#[bitsize(8)]
pub struct Register2 {
    vref: Vref,
    fir_filter: FirFilter,
    low_side_power: LowSidePower,
    idac_current: IdacCurrent,
}

/// IDAC1 routing configuration
/// These bits select the channel where IDAC1 is routed to.
#[bitsize(3)]
#[derive(FromBits)]
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

/// IDAC2 routing configuration
/// These bits select the channel where IDAC2 is routed to.
#[bitsize(3)]
#[derive(FromBits)]
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

/// DRDY mode
/// This bit controls the behavior of the DOUT/nDRDY pin when new data are ready.
#[bitsize(1)]
#[derive(FromBits)]
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

#[bitsize(8)]
pub struct Register3 {
    idac1_routing: Idac1Routing,
    idac2_routing: Idac2Routing,
    drdy_mode: DrdyMode,
    /// reserved value, must always be 0
    // bilge has automagic for fields called "reserved" or "padding", so it's not
    // in the ::new constructor
    reserved: bool,
}
