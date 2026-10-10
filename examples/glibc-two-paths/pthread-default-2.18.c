/* Selected glibc 2.18 boundary, SPDX-License-Identifier: MIT. */
#define _GNU_SOURCE
#include <pthread.h>
#include <stdio.h>
/* Explicit declarations allow the negative fixture to compile with 2.17 headers. */
extern int pthread_getattr_default_np(pthread_attr_t *);
extern int pthread_setattr_default_np(const pthread_attr_t *);
int main(void) {
    pthread_attr_t original, candidate, measured;
    size_t initial, changed;
    if (pthread_getattr_default_np(&original)) return 1;
    if (pthread_getattr_default_np(&candidate)) { pthread_attr_destroy(&original); return 2; }
    int failed = pthread_attr_getstacksize(&original, &initial);
    if (!failed && initial <= (size_t)-1 / 2) {
        changed = initial * 2;
        failed = pthread_attr_setstacksize(&candidate, changed);
        if (!failed) failed = pthread_setattr_default_np(&candidate);
        if (!failed) {
            failed = pthread_getattr_default_np(&measured);
            if (!failed) {
                size_t actual;
                failed = pthread_attr_getstacksize(&measured, &actual) || actual != changed;
                if (pthread_attr_destroy(&measured)) failed = 1;
            }
        }
    } else { failed = 1; }
    if (pthread_setattr_default_np(&original)) failed = 1;
    if (pthread_attr_destroy(&candidate)) failed = 1;
    if (pthread_attr_destroy(&original)) failed = 1;
    if (failed) return 3;
    puts("glibc 2.18 pthread defaults work!"); return 0;
}
