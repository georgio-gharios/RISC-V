// Minimal riscv-tests environment for rv-doom: bare machine, no CSRs, no traps.
// Test sources are used unmodified; only these macros differ from env/p.
//   pass: a0 = 0,                         ecall
//   fail: a0 = (TESTNUM << 1) | 1 (odd),  ecall
#ifndef _ENV_RVDOOM_H
#define _ENV_RVDOOM_H

#define TESTNUM gp

#define RVTEST_RV32U .macro init; .endm
#define RVTEST_RV64U .macro init; .endm
#define RVTEST_RV32M .macro init; .endm
#define RVTEST_RV64M .macro init; .endm

#define RVTEST_CODE_BEGIN       \
        .section .text.init;    \
        .align 6;               \
        .globl _start;          \
_start:                         \
        li TESTNUM, 0;          \
        init;

#define RVTEST_CODE_END unimp

#define RVTEST_PASS             \
        fence;                  \
        li a0, 0;               \
        ecall

#define RVTEST_FAIL             \
        fence;                  \
        slli a0, TESTNUM, 1;    \
        ori a0, a0, 1;          \
        ecall

#define EXTRA_DATA
#define RVTEST_DATA_BEGIN EXTRA_DATA .align 4;
#define RVTEST_DATA_END

#endif
