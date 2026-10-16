//! The hart: registers, pc, and the execute step.

use crate::decode::{decode, AluOp, BranchOp, Instr, LoadOp, StoreOp};
use crate::mem::{Bus, BusError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Halt {
    /// `ecall`: our convention is "exit with code a0".
    Exit(u32),
    Ebreak,
    IllegalInstruction { pc: u32, word: u32 },
    BusFault { pc: u32, addr: u32 },
}

pub struct Cpu {
    pub regs: [u32; 32],
    pub pc: u32,
    pub bus: Bus,
    pub instret: u64,
}

fn alu(op: AluOp, a: u32, b: u32) -> u32 {
    match op {
        AluOp::Add => a.wrapping_add(b),
        AluOp::Sub => a.wrapping_sub(b),
        AluOp::Sll => a << (b & 31),
        AluOp::Srl => a >> (b & 31),
        AluOp::Sra => ((a as i32) >> (b & 31)) as u32,
        AluOp::Slt => ((a as i32) < (b as i32)) as u32,
        AluOp::Sltu => (a < b) as u32,
        AluOp::Xor => a ^ b,
        AluOp::Or => a | b,
        AluOp::And => a & b,

        // M extension. The high-half multiplies widen to 64 bits first.
        AluOp::Mul => a.wrapping_mul(b),
        AluOp::Mulh => ((a as i32 as i64 * b as i32 as i64) >> 32) as u32,
        AluOp::Mulhsu => ((a as i32 as i64 * b as i64) >> 32) as u32,
        AluOp::Mulhu => ((a as u64 * b as u64) >> 32) as u32,
        // Division never traps in RISC-V. Divide by zero: quotient = all ones,
        // remainder = dividend. Overflow (i32::MIN / -1): quotient = i32::MIN, remainder = 0.
        AluOp::Div => match b {
            0 => u32::MAX,
            _ => (a as i32).wrapping_div(b as i32) as u32,
        },
        AluOp::Divu => a.checked_div(b).unwrap_or(u32::MAX),
        AluOp::Rem => match b {
            0 => a,
            _ => (a as i32).wrapping_rem(b as i32) as u32,
        },
        AluOp::Remu => a.checked_rem(b).unwrap_or(a),
    }
}

fn branch_taken(op: BranchOp, a: u32, b: u32) -> bool {
    match op {
        BranchOp::Eq => a == b,
        BranchOp::Ne => a != b,
        BranchOp::Lt => (a as i32) < (b as i32),
        BranchOp::Ge => (a as i32) >= (b as i32),
        BranchOp::Ltu => a < b,
        BranchOp::Geu => a >= b,
    }
}

impl Cpu {
    pub fn new(bus: Bus, pc: u32) -> Self {
        Cpu { regs: [0; 32], pc, bus, instret: 0 }
    }

    fn set(&mut self, rd: usize, value: u32) {
        if rd != 0 { self.regs[rd] = value; } // x0 is hard-wired to zero
    }

    /// Fetch, decode, execute one instruction.
    pub fn step(&mut self) -> Result<(), Halt> {
        let pc = self.pc;
        let fault = |BusError(addr)| Halt::BusFault { pc, addr };

        let word = self.bus.read(pc, 4).map_err(fault)?;
        let instr = decode(word).ok_or(Halt::IllegalInstruction { pc, word })?;
        let mut next_pc = pc.wrapping_add(4);

        match instr {
            Instr::Lui { rd, imm } => self.set(rd, imm as u32),
            Instr::Auipc { rd, imm } => self.set(rd, pc.wrapping_add(imm as u32)),
            Instr::Jal { rd, imm } => {
                self.set(rd, next_pc);
                next_pc = pc.wrapping_add(imm as u32);
            }
            Instr::Jalr { rd, rs1, imm } => {
                let target = self.regs[rs1].wrapping_add(imm as u32) & !1; // before writing rd (rd may equal rs1)
                self.set(rd, next_pc);
                next_pc = target;
            }
            Instr::Branch { op, rs1, rs2, imm } => {
                if branch_taken(op, self.regs[rs1], self.regs[rs2]) {
                    next_pc = pc.wrapping_add(imm as u32);
                }
            }
            Instr::Load { op, rd, rs1, imm } => {
                let addr = self.regs[rs1].wrapping_add(imm as u32);
                let value = match op {
                    LoadOp::Lb => self.bus.read(addr, 1).map_err(fault)? as i8 as i32 as u32,
                    LoadOp::Lh => self.bus.read(addr, 2).map_err(fault)? as i16 as i32 as u32,
                    LoadOp::Lw => self.bus.read(addr, 4).map_err(fault)?,
                    LoadOp::Lbu => self.bus.read(addr, 1).map_err(fault)?,
                    LoadOp::Lhu => self.bus.read(addr, 2).map_err(fault)?,
                };
                self.set(rd, value);
            }
            Instr::Store { op, rs1, rs2, imm } => {
                let addr = self.regs[rs1].wrapping_add(imm as u32);
                let width = match op { StoreOp::Sb => 1, StoreOp::Sh => 2, StoreOp::Sw => 4 };
                self.bus.write(addr, self.regs[rs2], width).map_err(fault)?;
            }
            Instr::OpImm { op, rd, rs1, imm } => self.set(rd, alu(op, self.regs[rs1], imm as u32)),
            Instr::Op { op, rd, rs1, rs2 } => self.set(rd, alu(op, self.regs[rs1], self.regs[rs2])),
            Instr::Fence => {}
            Instr::Ecall => return Err(Halt::Exit(self.regs[10])),
            Instr::Ebreak => return Err(Halt::Ebreak),
        }

        self.pc = next_pc;
        self.instret += 1;
        Ok(())
    }

    /// Run until the program halts or `max_steps` instructions have executed.
    pub fn run(&mut self, max_steps: u64) -> Option<Halt> {
        for _ in 0..max_steps {
            if let Err(h) = self.step() { return Some(h); }
        }
        None
    }
}
