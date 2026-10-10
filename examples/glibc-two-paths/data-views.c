#include <stdio.h>
#include <errno.h>

extern const char *const legacy_errors[125];
extern const char *const current_errors[135];
__asm__(".symver legacy_errors,sys_errlist@GLIBC_2.2.5");
__asm__(".symver current_errors,sys_errlist@GLIBC_2.12");
_Static_assert(sizeof legacy_errors == 1000, "x86_64 legacy ABI size");
_Static_assert(sizeof current_errors == 1080, "x86_64 current ABI size");

int main(void)
{
    for (unsigned i = 0; i < 125; ++i) {
        if (legacy_errors[i] != current_errors[i]) return 1;
    }
    /* Reserved errno slots can be NULL. Check a defined tail entry instead. */
    if (current_errors[EHWPOISON] == 0 || current_errors[0] == 0) return 2;
    return puts("Historical array sizes and values work!") < 0;
}
