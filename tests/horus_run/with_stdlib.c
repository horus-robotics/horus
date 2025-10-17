#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main() {
    char *str = malloc(100);
    strcpy(str, "Memory allocation works");
    printf("%s\n", str);
    free(str);
    return 0;
}
