// doomgeneric platform layer for rv-doom. Devices: see README "Memory map".
#include <stdint.h>
#include "doomgeneric.h"

#define FB        ((uint32_t *)0x20000000)
#define FB_CTRL   (*(volatile uint32_t *)0x13000000)
#define TIMER_MS  (*(volatile uint32_t *)0x11000000)
#define KEYBOARD  (*(volatile uint32_t *)0x12000000)

void DG_Init(void) {
    // Draw straight into the device framebuffer (same 320x200 0x00RRGGBB format), no copy.
    DG_ScreenBuffer = FB;
}

void DG_DrawFrame(void) { FB_CTRL = 1; }  // frame finished

void DG_SleepMs(uint32_t ms) {
    uint32_t start = TIMER_MS;
    while (TIMER_MS - start < ms) {}
}

uint32_t DG_GetTicksMs(void) { return TIMER_MS; }

int DG_GetKey(int *pressed, unsigned char *key) {
    uint32_t ev = KEYBOARD;  // 0 = none, else (pressed << 8) | doom keycode
    if (!ev) return 0;
    *pressed = ev >> 8;
    *key = ev & 0xff;
    return 1;
}

void DG_SetWindowTitle(const char *title) { (void)title; }

int main(void) {
    char *argv[] = { "doom", "-iwad", "doom1.wad", 0 };
    doomgeneric_Create(3, argv);
    for (;;) doomgeneric_Tick();
}
