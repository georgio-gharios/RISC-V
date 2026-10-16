// Tiny runtime for bare-metal programs on rv-doom. No libc.
#pragma once

void uart_putc(char c);
void print(const char *s);
void print_int(int v);
void print_hex(unsigned v);
