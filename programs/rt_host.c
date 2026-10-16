// Host version of rt.c, used only to produce reference output (make expected).
#include <stdio.h>
#include "rt.h"

void uart_putc(char c) { putchar(c); }
void print(const char *s) { fputs(s, stdout); }
void print_int(int v) { printf("%d", v); }
void print_hex(unsigned v) { printf("0x%08x", v); }
