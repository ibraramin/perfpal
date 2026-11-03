#include <stdlib.h>

int c_nested(int n) {
    int s = 0;
    for (int i = 0; i < n; i++) {
        for (int j = 0; j < n; j++) {
            s += i * j;
        }
    }
    return s;
}

int c_do_while(int n) {
    int s = 0;
    do { s++; } while (s < n);
    return s;
}

int c_empty(int n) {
    while (n > 0) { }
    return n;
}

int c_single(int n) {
    int s = 0;
    for (int i = 0; i < n; i++) s += i;
    return s;
}
