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

#[derive(Clone, Copy, Debug)]
pub struct Address(pub u16);

impl Address {
    pub const ZERO: Self = Self(0);
}
