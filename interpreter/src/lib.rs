use std::{collections::VecDeque, ops::ControlFlow};

use simplez_common::*;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionContext {
    #[serde(skip)]
    pub acc: u16,
    #[serde(skip)]
    pub pc: Address,
    #[serde(skip)]
    pub ir: u16,
    #[serde(with = "simplez_common::util::arrays")]
    memory: [u16; 512],
    #[serde(skip)]
    /// A list of the latest modified addresses.
    last_modifications: VecDeque<Address>,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            acc: 0,
            pc: Address(0),
            ir: 0,
            memory: [0; 512],
            last_modifications: Default::default(),
        }
    }
}

impl ExecutionContext {
    /// Steps the Simplez execution context by one instruction.
    pub fn step(&mut self) -> ControlFlow<(), ()> {
        self.ir = self.memory[self.pc.0 as usize];
        match Instruction::from(self.ir) {
            Instruction::Store { address } => self.set_addr(address, self.acc),
            Instruction::Load { address } => self.acc = self.memory[address.0 as usize],
            Instruction::Add { address } => {
                self.acc += self.memory[address.0 as usize];
                self.acc &= 0o777;
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
            Instruction::Clear => self.acc = 0,
            Instruction::Decrease => {
                self.acc -= 1;
                self.acc &= 0o777;
            }
            Instruction::Halt => return ControlFlow::Break(()),
        }
        self.pc.0 += 1;

        ControlFlow::Continue(())
    }

    pub fn reset_registers(&mut self) {
        self.acc = 0;
        self.pc = Address(0);
        self.ir = 0;
    }

    pub fn memory(&self) -> [u16; 512] {
        self.memory
    }

    pub fn set_addr(&mut self, addr: Address, val: u16) {
        self.memory[addr.0 as usize] = val;
        self.last_modifications.push_front(addr);
    }

    pub fn set_memory(&mut self, mem: [u16; 512]) {
        self.memory = mem;
        self.last_modifications.clear();
    }

    /// The zero bit register. Only set to true if `self.acc == 0`.
    pub fn zero(&self) -> bool {
        self.acc == 0
    }

    pub fn last_modifications(&self) -> &VecDeque<Address> {
        &self.last_modifications
    }
}
