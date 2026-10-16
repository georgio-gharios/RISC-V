//! Disassembler for the browser side panel: `Instr` -> assembly text (no pseudo-instructions).

use crate::decode::{decode, AluOp, BranchOp, Instr, LoadOp, StoreOp};

pub const ABI_NAMES: [&str; 32] = [
    "zero", "ra", "sp", "gp", "tp", "t0", "t1", "t2", "s0", "s1", "a0", "a1", "a2", "a3", "a4", "a5",
    "a6", "a7", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11", "t3", "t4", "t5", "t6",
];

fn alu_name(op: AluOp) -> &'static str {
    use AluOp::*;
    match op {
        Add => "add", Sub => "sub", Sll => "sll", Slt => "slt", Sltu => "sltu", Xor => "xor",
        Srl => "srl", Sra => "sra", Or => "or", And => "and", Mul => "mul", Mulh => "mulh",
        Mulhsu => "mulhsu", Mulhu => "mulhu", Div => "div", Divu => "divu", Rem => "rem", Remu => "remu",
    }
}

/// Disassemble `word` located at `pc` (branch/jump targets are shown as absolute addresses).
pub fn disasm(word: u32, pc: u32) -> String {
    let r = |i: usize| ABI_NAMES[i];
    let target = |imm: i32| pc.wrapping_add(imm as u32);
    let Some(instr) = decode(word) else { return format!(".word {word:#010x}") };
    match instr {
        Instr::Lui { rd, imm } => format!("lui {}, {:#x}", r(rd), (imm as u32) >> 12),
        Instr::Auipc { rd, imm } => format!("auipc {}, {:#x}", r(rd), (imm as u32) >> 12),
        Instr::Jal { rd, imm } => format!("jal {}, {:#x}", r(rd), target(imm)),
        Instr::Jalr { rd, rs1, imm } => format!("jalr {}, {}({})", r(rd), imm, r(rs1)),
        Instr::Branch { op, rs1, rs2, imm } => {
            let name = match op {
                BranchOp::Eq => "beq", BranchOp::Ne => "bne", BranchOp::Lt => "blt",
                BranchOp::Ge => "bge", BranchOp::Ltu => "bltu", BranchOp::Geu => "bgeu",
            };
            format!("{name} {}, {}, {:#x}", r(rs1), r(rs2), target(imm))
        }
        Instr::Load { op, rd, rs1, imm } => {
            let name = match op {
                LoadOp::Lb => "lb", LoadOp::Lh => "lh", LoadOp::Lw => "lw",
                LoadOp::Lbu => "lbu", LoadOp::Lhu => "lhu",
            };
            format!("{name} {}, {}({})", r(rd), imm, r(rs1))
        }
        Instr::Store { op, rs1, rs2, imm } => {
            let name = match op { StoreOp::Sb => "sb", StoreOp::Sh => "sh", StoreOp::Sw => "sw" };
            format!("{name} {}, {}({})", r(rs2), imm, r(rs1))
        }
        Instr::OpImm { op, rd, rs1, imm } => {
            let name = match op { AluOp::Sltu => "sltiu".to_string(), op => format!("{}i", alu_name(op)) };
            format!("{name} {}, {}, {}", r(rd), r(rs1), imm)
        }
        Instr::Op { op, rd, rs1, rs2 } => format!("{} {}, {}, {}", alu_name(op), r(rd), r(rs1), r(rs2)),
        Instr::Fence => "fence".into(),
        Instr::Ecall => "ecall".into(),
        Instr::Ebreak => "ebreak".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::disasm;

    #[test]
    fn matches_llvm_objdump_style() {
        // Expected strings checked against `llvm-objdump -d -M no-aliases` on programs/hello_asm.elf.
        assert_eq!(disasm(0x00001417, 0x8000_0000), "auipc s0, 0x1");
        assert_eq!(disasm(0x03440413, 0x8000_0004), "addi s0, s0, 52");
        assert_eq!(disasm(0x100004b7, 0x8000_0008), "lui s1, 0x10000");
        assert_eq!(disasm(0x00044503, 0x8000_000c), "lbu a0, 0(s0)");
        assert_eq!(disasm(0x00050a63, 0x8000_0010), "beq a0, zero, 0x80000024");
        assert_eq!(disasm(0x018080e7, 0x8000_0018), "jalr ra, 24(ra)");
        assert_eq!(disasm(0xfedff06f, 0x8000_0020), "jal zero, 0x8000000c");
        assert_eq!(disasm(0x00a48023, 0x8000_0030), "sb a0, 0(s1)");
        assert_eq!(disasm(0x00000073, 0), "ecall");
        assert_eq!(disasm(0x02c58733, 0), "mul a4, a1, a2");
    }
}
