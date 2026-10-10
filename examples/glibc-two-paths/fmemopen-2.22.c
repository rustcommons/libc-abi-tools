#define _GNU_SOURCE
#include <stdio.h>
#include <string.h>

extern FILE *open_old(void *, size_t, const char *);
extern FILE *open_new(void *, size_t, const char *);
__asm__(".symver open_old,fmemopen@GLIBC_2.2.5");
__asm__(".symver open_new,fmemopen@GLIBC_2.22");

int main(void) {
    char old_buffer[] = "abc", new_buffer[] = "abc";
    FILE *streams[] = {open_old(old_buffer, sizeof old_buffer, "r"),
                       open_new(new_buffer, sizeof new_buffer, "r")};
    for (size_t i = 0; i < 2; ++i) {
        char output[3];
        if (!streams[i] || fread(output, 1, sizeof output, streams[i]) != sizeof output ||
            memcmp(output, "abc", sizeof output) != 0 || fclose(streams[i]) != 0)
            return 1;
    }
    puts("Both fmemopen versions work!");
    return 0;
}
