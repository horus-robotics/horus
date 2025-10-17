#include <stdio.h>
#include <stdlib.h>

int main() {
    char *val = getenv("HORUS_TEST_VAR");
    if (val != NULL) {
        printf("Got env var: %s\n", val);
    } else {
        printf("No env var found\n");
    }
    return 0;
}
