/* SPDX-License-Identifier: MIT */
#define _GNU_SOURCE
#include <stdio.h>
#if defined(TEST_M)
#include <math.h>
int main(void) {
    volatile double zero = 0.0, four = 4.0, two = 2.0, three = 3.0;
    double s = sin(zero), c = cos(zero), r = sqrt(four), p = pow(two, three);
    if (!isfinite(s) || !isfinite(c) || !isfinite(r) || !isfinite(p) ||
        fabs(s) > 1e-12 || fabs(c - 1.0) > 1e-12 ||
        fabs(r - 2.0) > 1e-12 || fabs(p - 8.0) > 1e-12) return 1;
    puts("m behavior passed"); return 0;
}
#elif defined(TEST_DL)
#include <dlfcn.h>
int main(int argc, char **argv) {
    if (argc != 2) return 1;
    void *handle = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!handle) { fprintf(stderr, "%s\n", dlerror()); return 2; }
    dlerror();
    int (*ordinary)(void) = (int (*)(void))dlsym(handle, "fixture_value");
    const char *error = dlerror();
    if (error || !ordinary || ordinary() != 42) { dlclose(handle); return 3; }
    int (*versioned)(void) = (int (*)(void))dlvsym(handle, "fixture_value", "FIXTURE_1.0");
    error = dlerror();
    if (error || !versioned || versioned() != 42) { dlclose(handle); return 4; }
    dlerror();
    void *missing = dlvsym(handle, "fixture_value", "FIXTURE_2.0");
    error = dlerror();
    if (missing || !error) { dlclose(handle); return 5; }
    if (dlclose(handle)) return 6;
    puts("dl behavior passed"); return 0;
}
#elif defined(TEST_RT)
#include <signal.h>
#include <time.h>
int main(void) {
    struct timespec first, second, resolution;
    struct sigevent event = {0};
    timer_t timer;
    event.sigev_notify = SIGEV_NONE;
    if (clock_gettime(CLOCK_MONOTONIC, &first) || clock_getres(CLOCK_MONOTONIC, &resolution)) return 1;
    if (timer_create(CLOCK_MONOTONIC, &event, &timer)) return 2;
    if (timer_delete(timer)) return 3;
    if (clock_gettime(CLOCK_MONOTONIC, &second)) return 4;
    if (second.tv_sec < first.tv_sec || (second.tv_sec == first.tv_sec && second.tv_nsec < first.tv_nsec)
        || resolution.tv_nsec < 0 || resolution.tv_nsec >= 1000000000L) return 5;
    puts("rt behavior passed"); return 0;
}
#elif defined(TEST_UTIL)
#include <pty.h>
#include <termios.h>
#include <unistd.h>
int main(void) {
    int master, slave;
    struct termios settings;
    if (openpty(&master, &slave, NULL, NULL, NULL)) return 1;
    int failed = tcgetattr(slave, &settings) || !isatty(slave);
    if (close(slave)) failed = 1;
    if (close(master)) failed = 1;
    if (failed) return 2;
    puts("util behavior passed"); return 0;
}
#elif defined(TEST_RESOLV)
#include <arpa/nameser.h>
#include <resolv.h>
#include <string.h>
int main(void) {
    const unsigned char packet[] = {
        0x12,0x34, 0x01,0x00, 0x00,0x01, 0,0, 0,0, 0,0,
        7,'e','x','a','m','p','l','e',4,'t','e','s','t',0, 0,1,0,1
    };
    ns_msg message;
    ns_rr question;
    char name[NS_MAXDNAME];
    if (ns_initparse(packet, sizeof packet, &message) || ns_msg_id(message) != 0x1234
        || ns_msg_count(message, ns_s_qd) != 1) return 1;
    if (ns_parserr(&message, ns_s_qd, 0, &question) || strcmp(ns_rr_name(question), "example.test")
        || ns_rr_type(question) != ns_t_a || ns_rr_class(question) != ns_c_in) return 2;
    if (dn_expand(packet, packet + sizeof packet, packet + 12, name, sizeof name) != 14
        || strcmp(name, "example.test")) return 3;
    if (ns_initparse(packet, 5, &message) >= 0) return 4;
    puts("resolv behavior passed"); return 0;
}
#else
#error Select a library behavior test
#endif
