/* Link-only probe for an interface newer than the provider; never executed.
   TOO_NEW_SYMBOL comes from too-new-symbol.py (default: memfd_create, 2.27).
   An explicit declaration allows this link test to use the old headers. */
#ifndef TOO_NEW_SYMBOL
#define TOO_NEW_SYMBOL memfd_create
#endif
extern int TOO_NEW_SYMBOL(const char *name, unsigned int flags);

int main(void)
{
    return TOO_NEW_SYMBOL("glibc-version-negative-test", 0) < 0;
}
