/* rv-doom layout for Doom: like programs/link.ld, plus heap and init-array symbols for picolibc. */
OUTPUT_ARCH("riscv")
ENTRY(_start)
MEMORY { RAM (rwx) : ORIGIN = 0x80000000, LENGTH = 64M }
STACK_SIZE = 1M;
SECTIONS
{
  .text   : { *(.text.init) *(.text .text.*) } > RAM
  .rodata : { *(.rodata .rodata.* .srodata .srodata.*) } > RAM
  .init_array : {
    PROVIDE(__bothinit_array_start = .);
    PROVIDE(__init_array_start = .);
    KEEP(*(.preinit_array)) KEEP(*(SORT(.init_array.*))) KEEP(*(.init_array))
    PROVIDE(__init_array_end = .);
    PROVIDE(__bothinit_array_end = .);
  } > RAM
  .fini_array : {
    PROVIDE(__fini_array_start = .);
    KEEP(*(SORT(.fini_array.*))) KEEP(*(.fini_array))
    PROVIDE(__fini_array_end = .);
  } > RAM
  .data   : { *(.data .data.* .sdata .sdata.*) } > RAM
  .bss (NOLOAD) : ALIGN(4) {
    _bss_start = .;
    *(.bss .bss.* .sbss .sbss.* COMMON)
    . = ALIGN(4);
    _bss_end = .;
  } > RAM
  __heap_start = ALIGN(_bss_end, 16);
  _stack_top = ORIGIN(RAM) + LENGTH(RAM);
  __heap_end = _stack_top - STACK_SIZE;
}
