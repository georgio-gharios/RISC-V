// Runtime for the emulator: output goes to the UART at 0x10000000.
#include "rt.h"

#define UART ((volatile unsigned char *)0x10000000)

void uart_putc(char c) { UART[0] = c; }

void print(const char *s) {
    while (*s) uart_putc(*s++);
}

void print_int(int v) {
    char buf[12];
    int n = 0;
    unsigned u = v < 0 ? -(unsigned)v : (unsigned)v;
    do { buf[n++] = '0' + u % 10; u /= 10; } while (u);
    if (v < 0) uart_putc('-');
    while (n) uart_putc(buf[--n]);
}

void print_hex(unsigned v) {
    print("0x");
    for (int i = 28; i >= 0; i -= 4) uart_putc("0123456789abcdef"[(v >> i) & 0xf]);
}

// The compiler may emit calls to these even without a libc.
void *memset(void *dst, int c, unsigned n) {
    unsigned char *d = dst;
    while (n--) *d++ = (unsigned char)c;
    return dst;
}

void *memcpy(void *dst, const void *src, unsigned n) {
    unsigned char *d = dst;
    const unsigned char *s = src;
    while (n--) *d++ = *s++;
    return dst;
}
