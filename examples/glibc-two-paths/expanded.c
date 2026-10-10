#include <stdio.h>
#include <string.h>

extern void *copy_old(void *, const void *, size_t);
__asm__(".symver copy_old,memcpy@GLIBC_2.2.5");

int main(void)
{
    const char message[] = "Both memcpy versions and stdout work!\n";
    char old_copy[sizeof message];
    char new_copy[sizeof message];
    copy_old(old_copy, message, sizeof message);
    memcpy(new_copy, message, sizeof message);
    for (size_t i = 0; i < sizeof message; ++i) {
        if (old_copy[i] != message[i] || new_copy[i] != message[i]) return 1;
    }
    if (fwrite(new_copy, 1, sizeof message - 1, stdout) != sizeof message - 1) return 2;
    return fflush(stdout) != 0;
}
