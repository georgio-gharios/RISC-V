# Phase 1 test program: print "hello\n" via the UART, then exit(0) with ecall.
    .section .text
    .globl _start
_start:
    la    s0, msg            # auipc + addi
    li    s1, 0x10000000     # UART base (lui)
loop:
    lbu   a0, 0(s0)
    beqz  a0, done
    call  putc               # jal ra / auipc+jalr
    addi  s0, s0, 1
    j     loop
done:
    li    a0, 0              # exit code
    ecall

putc:                        # a0 = char
    sb    a0, 0(s1)
    ret                      # jalr x0, 0(ra)

    .section .rodata
msg:
    .asciz "hello\n"
