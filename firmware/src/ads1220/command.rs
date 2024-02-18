// I can't use Bilge here because Rreg/Wreg pack magic values with enums,
// and Bilge can't do that.

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Offset {
    Register0 = 0b00,
    Register1 = 0b01,
    Register2 = 0b10,
    Register3 = 0b11,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Length {
    L1 = 0b00,
    L2 = 0b01,
    L3 = 0b10,
    L4 = 0b11,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Command {
    Reset,
    StartOrSync,
    Powerdown,
    Rdata,
    Rreg(Offset, Length),
    Wreg(Offset, Length),
}

impl Into<u8> for Command {
    fn into(self) -> u8 {
        match self {
            Command::Reset => 0b0000_0110,
            Command::StartOrSync => 0b0000_1000,
            Command::Powerdown => 0b0000_1010,
            Command::Rdata => 0b0001_0000,
            Command::Rreg(offset, length) => 0b0010_0000 | (offset as u8) << 2 | length as u8,
            Command::Wreg(offset, length) => 0b0100_0000 | (offset as u8) << 2 | length as u8,
        }
    }
}
