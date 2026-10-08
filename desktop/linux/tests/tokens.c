#include <stdio.h>
#include <string.h>

#ifndef TOKENS_CSS
#error missing TOKENS_CSS
#endif

static int has_colour(const char *text, const char *name) {
    char needle[64];
    int wrote = snprintf(needle, sizeof(needle), "@define-color %s", name);
    if (wrote < 0 || (size_t)wrote >= sizeof(needle)) {
        return 0;
    }
    return strstr(text, needle) != NULL;
}

int main(void) {
    FILE *file = fopen(TOKENS_CSS, "r");
    char text[8192];
    size_t read;
    const char *names[] = {"window_bg", "accent", "foreground"};
    size_t index;
    if (file == NULL) {
        fprintf(stderr, "could not open the token file\n");
        return 1;
    }
    read = fread(text, 1, sizeof(text) - 1, file);
    if (ferror(file) != 0) {
        fclose(file);
        fprintf(stderr, "could not read the token file\n");
        return 1;
    }
    text[read] = '\0';
    fclose(file);
    for (index = 0; index < sizeof(names) / sizeof(names[0]); index++) {
        if (has_colour(text, names[index]) == 0) {
            fprintf(stderr, "missing colour %s\n", names[index]);
            return 1;
        }
    }
    return 0;
}
