#define _GNU_SOURCE
#include <stdio.h>
#include <string.h>
int main(void) {
    char buffer[] = "abc", output[3];
    FILE *stream = fmemopen(buffer, sizeof buffer, "r");
    if (!stream || fread(output, 1, sizeof output, stream) != sizeof output ||
        memcmp(output, "abc", sizeof output) != 0 || fclose(stream) != 0)
        return 1;
    puts("Default fmemopen interface works!");
    return 0;
}
