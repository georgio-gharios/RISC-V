// Sieve of Eratosthenes: list primes below 200, count primes below 100000.
#include "rt.h"

#define N 100000
static char composite[N];   // in .bss, zeroed by crt0

int main(void) {
    for (int i = 2; i * i < N; i++)
        if (!composite[i])
            for (int j = i * i; j < N; j += i) composite[j] = 1;

    print("primes below 200:");
    for (int i = 2; i < 200; i++)
        if (!composite[i]) { uart_putc(' '); print_int(i); }
    print("\n");

    int count = 0;
    for (int i = 2; i < N; i++) count += !composite[i];
    print("count below 100000: "); print_int(count); print("\n");
    return count == 9592 ? 0 : 1;
}
