// rv-doom memory-mapped devices (see README "Memory map").
#pragma once

#define FB_WIDTH  320
#define FB_HEIGHT 200
#define FB        ((volatile unsigned *)0x20000000)   // 0x00RRGGBB pixels, row-major
#define FB_CTRL   (*(volatile unsigned *)0x13000000)  // write: frame done; read: frame count
#define TIMER_MS  (*(volatile unsigned *)0x11000000)  // milliseconds since start
#define KEYBOARD  (*(volatile unsigned *)0x12000000)  // read pops: 0 or (pressed << 8) | key
