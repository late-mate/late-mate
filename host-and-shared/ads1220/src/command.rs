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

impl From<Command> for u8 {
    fn from(command: Command) -> Self {
        match command {
            Command::Reset => 0b0000_0110,
            Command::StartOrSync => 0b0000_1000,
            Command::Powerdown => 0b0000_1010,
            Command::Rdata => 0b0001_0000,
            Command::Rreg(offset, length) => 0b0010_0000 | (offset as u8) << 2 | length as u8,
            Command::Wreg(offset, length) => 0b0100_0000 | (offset as u8) << 2 | length as u8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wreg() {
        let command = Command::Wreg(Offset::Register0, Length::L2);
        assert_eq!(u8::from(command), 0b0100_0001u8, "the command must match the datasheet example");
    }

    #[test]
    fn test_rreg() {
        let command = Command::Rreg(Offset::Register1, Length::L3);
        assert_eq!(u8::from(command), 0b0010_0110, "the command must match the datasheet example");
    }
}