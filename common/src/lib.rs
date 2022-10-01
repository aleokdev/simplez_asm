use std::fmt::Display;

#[derive(Clone, Copy, Debug)]
pub enum Instruction<Addr = Address> {
    Store { address: Addr },
    Load { address: Addr },
    Add { address: Addr },
    Branch { address: Addr },
    BranchIfZero { address: Addr },
    Clear,
    Decrease,
    Halt,
}

impl From<u16> for Instruction<Address> {
    fn from(val: u16) -> Self {
        let ins = (val >> 9) & 0o7;
        let param = Address(val & 0o777);
        match ins {
            0 => Instruction::Store { address: param },
            1 => Instruction::Load { address: param },
            2 => Instruction::Add { address: param },
            3 => Instruction::Branch { address: param },
            4 => Instruction::BranchIfZero { address: param },
            5 => Instruction::Clear,
            6 => Instruction::Decrease,
            7 => Instruction::Halt,
            _ => unreachable!(),
        }
    }
}

impl Display for Instruction<Address> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Store { address } => f.write_fmt(format_args!("ST {}", address)),
            Instruction::Load { address } => f.write_fmt(format_args!("LD {}", address)),
            Instruction::Add { address } => f.write_fmt(format_args!("ADD {}", address)),
            Instruction::Branch { address } => f.write_fmt(format_args!("BR {}", address)),
            Instruction::BranchIfZero { address } => f.write_fmt(format_args!("BZ {}", address)),
            Instruction::Clear => f.write_str("CLR"),
            Instruction::Decrease => f.write_str("DEC"),
            Instruction::Halt => f.write_str("HALT"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Address(pub u16);

impl Address {
    pub const ZERO: Self = Self(0);
}

impl Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("/{}", self.0))
    }
}
