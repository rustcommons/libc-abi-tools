/* Selected glibc 2.21 default removal, SPDX-License-Identifier: MIT. */
#include <signal.h>
#include <stdio.h>
/* glibc 2.21 removed sigvec from its headers and kept only a compatibility
   symbol. Older headers still declare it, so reach the same unversioned
   symbol under a local name with the historical layout. */
struct legacy_sigvec { void (*sv_handler)(int); int sv_mask; int sv_flags; };
extern int legacy_sigvec(int, const struct legacy_sigvec *, struct legacy_sigvec *)
    __asm__("sigvec");
static volatile sig_atomic_t delivered;
static void handler(int signal_number) { delivered = signal_number; }
int main(void) {
    struct legacy_sigvec action = { handler, 0, 0 }, previous;
    if (legacy_sigvec(SIGUSR1, &action, &previous)) return 1;
    if (raise(SIGUSR1) || delivered != SIGUSR1) return 2;
    if (legacy_sigvec(SIGUSR1, &previous, 0)) return 3;
    puts("Default sigvec interface works!");
    return 0;
}
