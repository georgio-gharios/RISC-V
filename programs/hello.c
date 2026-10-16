#include "rt.h"

static int counter = 41;   // .data: must be loaded from the image
static int zeroed[16];     // .bss: must be zeroed by crt0

int main(void) {
    print("hello from C\n");
    counter++;
    print("counter = "); print_int(counter); print("\n");
    print("negative = "); print_int(-12345); print("\n");
    print("hex = "); print_hex(0xdeadbeef); print("\n");
    int sum = 0;
    for (int i = 0; i < 16; i++) sum += zeroed[i];
    print("bss sum = "); print_int(sum); print("\n");
    return 0;
}
