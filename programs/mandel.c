// ASCII Mandelbrot in Q12 fixed point (no floats: the core is RV32IM only).
#include "rt.h"

#define FRAC 12
#define ONE (1 << FRAC)
#define W 72
#define H 28
#define MAX_ITER 64

int main(void) {
    const char *shades = " .:-=+*#%@";
    for (int row = 0; row < H; row++) {
        int ci = -ONE * 6 / 5 + row * (ONE * 12 / 5) / (H - 1);      // -1.2 .. 1.2
        for (int col = 0; col < W; col++) {
            int cr = -ONE * 2 + col * (ONE * 3) / (W - 1);           // -2.0 .. 1.0
            int zr = 0, zi = 0, n = 0;
            while (n < MAX_ITER) {
                int zr2 = (zr * zr) >> FRAC, zi2 = (zi * zi) >> FRAC;
                if (zr2 + zi2 > 4 * ONE) break;
                zi = ((zr * zi) >> (FRAC - 1)) + ci;                 // 2*zr*zi + ci
                zr = zr2 - zi2 + cr;
                n++;
            }
            uart_putc(n == MAX_ITER ? '@' : shades[n % 9]);
        }
        uart_putc('\n');
    }
    return 0;
}
