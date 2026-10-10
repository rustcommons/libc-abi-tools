#include <stdio.h>

int main(void)
{
    return puts("Hello from the glibc compatibility experiment!") < 0;
}
