// Exercises the timer and keyboard. tests/phase4.rs drives the timer and injects keys.
#include "devices.h"
#include "rt.h"

int main(void) {
    unsigned t0 = TIMER_MS;
    print("t0 = "); print_int(t0); print("\n");
    while (TIMER_MS < t0 + 100) {}                  // busy-wait 100 ms
    print("waited until "); print_int(TIMER_MS); print("\n");

    print("empty read = "); print_int(KEYBOARD); print("\n");
    print("waiting for keys\n");
    for (int got = 0; got < 3;) {
        unsigned ev = KEYBOARD;
        if (!ev) continue;
        print((ev >> 8) ? "down " : "up "); print_int(ev & 0xff); print("\n");
        got++;
    }
    return 0;
}
