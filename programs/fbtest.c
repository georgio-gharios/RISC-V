// Draws a known test pattern into the framebuffer and signals one frame.
// tests/phase4.rs recomputes the pattern independently and compares every pixel.
#include "devices.h"
#include "rt.h"

static const unsigned bars[8] = {
    0xffffff, 0xffff00, 0x00ffff, 0x00ff00, 0xff00ff, 0xff0000, 0x0000ff, 0x000000,
};

static void put(int x, int y, unsigned c) { FB[y * FB_WIDTH + x] = c; }

int main(void) {
    for (int y = 0; y < FB_HEIGHT; y++)
        for (int x = 0; x < FB_WIDTH; x++) {
            unsigned c;
            if (y < 40)
                c = bars[x / 40];                                  // 8 color bars
            else
                c = ((x * 255 / 319) << 16) | ((y * 255 / 199) << 8) | 0x40;  // gradient
            put(x, y, c);
        }
    for (int i = 40; i < FB_HEIGHT; i++) put(i + 60, i, 0xff0000); // red diagonal
    for (int x = 0; x < FB_WIDTH; x++) { put(x, 0, 0xffffff); put(x, FB_HEIGHT - 1, 0xffffff); }
    for (int y = 0; y < FB_HEIGHT; y++) { put(0, y, 0xffffff); put(FB_WIDTH - 1, y, 0xffffff); }

    FB_CTRL = 1;
    print("frames = "); print_int(FB_CTRL); print("\n");
    return FB_CTRL == 1 ? 0 : 1;
}
