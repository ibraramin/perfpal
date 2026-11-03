#include <stdio.h>
#include <stdlib.h>

void dead_sites(int n) {
    for (int i = 0; i < n; i++) {
        printf("live\n");
        malloc(4);
    }
#if 0
    for (int i = 0; i < n; i++) {
        printf("dead\n");
        malloc(8);
    }
#endif
}
