use std::fmt::Display;
use std::ops::{Index, IndexMut};

use twelve_bit::u12;
use twelve_bit::u12::*;

pub mod util;

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
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

impl From<U12> for Instruction<Address> {
    fn from(val: U12) -> Self {
        let ins: U12 = (val >> 9) & u12!(0o7);
        let param = Address(val & u12!(0o777));
        match u16::from(ins) {
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Address(pub U12);

impl Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("/{}", u16::from(self.0)))
    }
}

#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Memory(#[serde(with = "util::arrays")] pub [U12; 512]);

impl Default for Memory {
    fn default() -> Self {
        Self([u12!(0); 512])
    }
}

impl Index<Address> for Memory {
    type Output = U12;

    fn index(&self, index: Address) -> &Self::Output {
        &self.0[usize::from(index.0)]
    }
}

impl IndexMut<Address> for Memory {
    fn index_mut(&mut self, index: Address) -> &mut Self::Output {
        &mut self.0[usize::from(index.0)]
    }
}

impl Memory {
    pub fn iter(&self) -> std::slice::Iter<U12> {
        self.0.iter()
    }
}
