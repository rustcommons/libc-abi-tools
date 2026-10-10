#include <stdio.h>

extern int foo(void), dispatch(void), weak_function(void);
extern int old_foo(void);
__asm__(".symver old_foo,foo@GLIBC_2.2.5");
extern __thread int tls_data;
extern void *shared_data, *weak_data;
int imported_symbol = 7;

int main(void)
{
    if (old_foo() != 1 || foo() != 2 || dispatch() != 2 || weak_function() != 2) return 1;
    if (tls_data != 0) return 2;
    tls_data = 17;
    if (tls_data != 17) return 3;
    if (shared_data != &imported_symbol || weak_data != shared_data) return 4;
    if (&shared_data != &weak_data) return 5;
    return puts("Fixture versions, IFUNC, TLS, and alias storage work!") < 0;
}
