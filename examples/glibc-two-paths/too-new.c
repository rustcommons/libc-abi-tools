/* Explicit declaration allows this link test to use the old headers. */
extern int memfd_create(const char *name, unsigned int flags);

int main(void)
{
    return memfd_create("glibc-version-negative-test", 0) < 0;
}
