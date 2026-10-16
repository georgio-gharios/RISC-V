//! Instruction decoder: turns a 32-bit word into an `Instr`.
//!
//! RV32 base formats (bit 31 on the left):
//!   R: funct7 | rs2 | rs1 | funct3 | rd | opcode
//!   I:   imm[11:0]  | rs1 | funct3 | rd | opcode
//!   S: imm[11:5] | rs2 | rs1 | funct3 | imm[4:0] | opcode
//!   B: like S, but the immediate bits are shuffled and it counts in 2-byte units
//!   U:   imm[31:12]                     | rd | opcode
//!   J: like U, but the immediate bits are shuffled and it counts in 2-byte units

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AluOp { Add, Sub, Sll, Slt, Sltu, Xor, Srl, Sra, Or, And }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchOp { Eq, Ne, Lt, Ge, Ltu, Geu }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadOp { Lb, Lh, Lw, Lbu, Lhu }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreOp { Sb, Sh, Sw }

/// A decoded instruction. Register fields are indices 0..=31.
/// Immediates are already sign-extended to i32.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instr {
    Lui   { rd: usize, imm: i32 },
    Auipc { rd: usize, imm: i32 },
    Jal   { rd: usize, imm: i32 },
    Jalr  { rd: usize, rs1: usize, imm: i32 },
    Branch { op: BranchOp, rs1: usize, rs2: usize, imm: i32 },
    Load   { op: LoadOp,  rd: usize, rs1: usize, imm: i32 },
    Store  { op: StoreOp, rs1: usize, rs2: usize, imm: i32 },
    OpImm  { op: AluOp, rd: usize, rs1: usize, imm: i32 },   // addi, slli, ...
    Op     { op: AluOp, rd: usize, rs1: usize, rs2: usize }, // add, sll, ...
    Fence,
    Ecall,
    Ebreak,
}

// --- field extractors ---
fn opcode(i: u32) -> u32 { i & 0x7f }
fn rd(i: u32) -> usize { ((i >> 7) & 0x1f) as usize }
fn funct3(i: u32) -> u32 { (i >> 12) & 0x7 }
fn rs1(i: u32) -> usize { ((i >> 15) & 0x1f) as usize }
fn rs2(i: u32) -> usize { ((i >> 20) & 0x1f) as usize }
fn funct7(i: u32) -> u32 { i >> 25 }

// --- immediates (sign-extended by shifting the i32 arithmetically) ---
fn imm_i(i: u32) -> i32 { (i as i32) >> 20 }
fn imm_s(i: u32) -> i32 { ((i as i32) >> 25 << 5) | ((i >> 7) & 0x1f) as i32 }
fn imm_u(i: u32) -> i32 { (i & 0xffff_f000) as i32 }
fn imm_b(i: u32) -> i32 {
    ((i as i32) >> 31 << 12)            // imm[12]   <- bit 31 (sign)
        | (((i >> 7) & 1) << 11) as i32  // imm[11]   <- bit 7
        | (((i >> 25) & 0x3f) << 5) as i32 // imm[10:5] <- bits 30:25
        | (((i >> 8) & 0xf) << 1) as i32 // imm[4:1]  <- bits 11:8
}
fn imm_j(i: u32) -> i32 {
    ((i as i32) >> 31 << 20)              // imm[20]    <- bit 31 (sign)
        | (i & 0x000f_f000) as i32         // imm[19:12] <- bits 19:12
        | (((i >> 20) & 1) << 11) as i32   // imm[11]    <- bit 20
        | (((i >> 21) & 0x3ff) << 1) as i32 // imm[10:1]  <- bits 30:21
}

/// Decode one instruction word. Returns `None` for anything we don't implement.
pub fn decode(i: u32) -> Option<Instr> {
    use Instr::*;
    let (rd, rs1, rs2, f3, f7) = (rd(i), rs1(i), rs2(i), funct3(i), funct7(i));

    let instr = match opcode(i) {
        0b0110111 => Lui { rd, imm: imm_u(i) },
        0b0010111 => Auipc { rd, imm: imm_u(i) },
        0b1101111 => Jal { rd, imm: imm_j(i) },
        0b1100111 if f3 == 0 => Jalr { rd, rs1, imm: imm_i(i) },

        0b1100011 => {
            let op = match f3 {
                0 => BranchOp::Eq,  1 => BranchOp::Ne,
                4 => BranchOp::Lt,  5 => BranchOp::Ge,
                6 => BranchOp::Ltu, 7 => BranchOp::Geu,
                _ => return None,
            };
            Branch { op, rs1, rs2, imm: imm_b(i) }
        }

        0b0000011 => {
            let op = match f3 {
                0 => LoadOp::Lb, 1 => LoadOp::Lh, 2 => LoadOp::Lw,
                4 => LoadOp::Lbu, 5 => LoadOp::Lhu,
                _ => return None,
            };
            Load { op, rd, rs1, imm: imm_i(i) }
        }

        0b0100011 => {
            let op = match f3 {
                0 => StoreOp::Sb, 1 => StoreOp::Sh, 2 => StoreOp::Sw,
                _ => return None,
            };
            Store { op, rs1, rs2, imm: imm_s(i) }
        }

        0b0010011 => {
            let imm = imm_i(i);
            let op = match (f3, f7) {
                (0, _) => AluOp::Add,
                (2, _) => AluOp::Slt,
                (3, _) => AluOp::Sltu,
                (4, _) => AluOp::Xor,
                (6, _) => AluOp::Or,
                (7, _) => AluOp::And,
                // Shifts: the immediate is a 5-bit shamt, funct7 picks logical/arithmetic.
                (1, 0x00) => AluOp::Sll,
                (5, 0x00) => AluOp::Srl,
                (5, 0x20) => AluOp::Sra,
                _ => return None,
            };
            let imm = if matches!(f3, 1 | 5) { imm & 0x1f } else { imm };
            OpImm { op, rd, rs1, imm }
        }

        0b0110011 => {
            let op = match (f3, f7) {
                (0, 0x00) => AluOp::Add,
                (0, 0x20) => AluOp::Sub,
                (1, 0x00) => AluOp::Sll,
                (2, 0x00) => AluOp::Slt,
                (3, 0x00) => AluOp::Sltu,
                (4, 0x00) => AluOp::Xor,
                (5, 0x00) => AluOp::Srl,
                (5, 0x20) => AluOp::Sra,
                (6, 0x00) => AluOp::Or,
                (7, 0x00) => AluOp::And,
                _ => return None,
            };
            Op { op, rd, rs1, rs2 }
        }

        0b0001111 => Fence, // fence / fence.i: no-ops on a single in-order hart
        0b1110011 => match i {
            0x0000_0073 => Ecall,
            0x0010_0073 => Ebreak,
            _ => return None,
        },
        _ => return None,
    };
    Some(instr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_known_words() {
        // addi a0, zero, -1
        assert_eq!(decode(0xfff00513), Some(Instr::OpImm { op: AluOp::Add, rd: 10, rs1: 0, imm: -1 }));
        // lui a0, 0x10000
        assert_eq!(decode(0x10000537), Some(Instr::Lui { rd: 10, imm: 0x1000_0000 }));
        // sb a1, 0(a0)
        assert_eq!(decode(0x00b50023), Some(Instr::Store { op: StoreOp::Sb, rs1: 10, rs2: 11, imm: 0 }));
        // beq zero, zero, -4
        assert_eq!(decode(0xfe000ee3), Some(Instr::Branch { op: BranchOp::Eq, rs1: 0, rs2: 0, imm: -4 }));
        // jal ra, -8
        assert_eq!(decode(0xff9ff0ef), Some(Instr::Jal { rd: 1, imm: -8 }));
        // srai a0, a0, 3
        assert_eq!(decode(0x40355513), Some(Instr::OpImm { op: AluOp::Sra, rd: 10, rs1: 10, imm: 3 }));
        // sw a1, -4(sp)
        assert_eq!(decode(0xfeb12e23), Some(Instr::Store { op: StoreOp::Sw, rs1: 2, rs2: 11, imm: -4 }));
        assert_eq!(decode(0), None);
    }
}
