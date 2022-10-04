use std::{collections::VecDeque, ops::ControlFlow};

use simplez_common::*;
use twelve_bit::u12;
use twelve_bit::u12::*;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionContext {
    #[serde(skip)]
    pub acc: U12,
    #[serde(skip)]
    pub pc: Address,
    #[serde(skip)]
    pub ir: U12,
    memory: Memory,
    #[serde(skip)]
    /// A list of the latest modified addresses.
    last_modifications: VecDeque<Address>,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            acc: u12!(0),
            pc: Default::default(),
            ir: u12!(0),
            memory: Default::default(),
            last_modifications: Default::default(),
        }
    }
}

impl ExecutionContext {
    /// Steps the Simplez execution context by one instruction.
    pub fn step(&mut self) -> ControlFlow<(), ()> {
        self.ir = self.memory[self.pc];
        match Instruction::from(self.ir) {
            Instruction::Store { address } => self.set_addr(address, self.acc),
            Instruction::Load { address } => self.acc = self.memory[address],
            Instruction::Add { address } => {
                self.acc += self.memory[address];
            }
            Instruction::Branch { address } => {
                self.pc = address;
                return ControlFlow::Continue(());
            }
            Instruction::BranchIfZero { address } => {
                if self.zero() {
                    self.pc = address;
                    return ControlFlow::Continue(());
                }
            }
            Instruction::Clear => self.acc = u12!(0),
            Instruction::Decrease => {
                self.acc -= u12!(1);
            }
            Instruction::Halt => return ControlFlow::Break(()),
        }
        self.pc.0 += u12!(1);

        ControlFlow::Continue(())
    }

    pub fn reset_registers(&mut self) {
        self.acc = Default::default();
        self.pc = Default::default();
        self.ir = Default::default();
    }

    pub fn memory(&self) -> &Memory {
        &self.memory
    }

    pub fn set_addr(&mut self, addr: Address, val: U12) {
        self.memory[addr] = val;
        self.last_modifications.push_front(addr);
    }

    pub fn set_memory(&mut self, mem: Memory) {
        self.memory = mem;
        self.last_modifications.clear();
    }

    /// The zero bit register. Only set to true if `self.acc == 0`.
    pub fn zero(&self) -> bool {
        self.acc == u12!(0)
    }

    pub fn last_modifications(&self) -> &VecDeque<Address> {
        &self.last_modifications
    }
}
