#include "library.h"
#include "mailune.h"

#include <stdio.h>
#include <string.h>

int main(void) {
    char *parsed;
    if (!mailune_linux_links()) {
        fprintf(stderr, "mailune_ready did not return a record\n");
        return 1;
    }
    parsed = mailune_parse_query("from:ada hello");
    if (parsed == NULL || strstr(parsed, "\"ok\"") == NULL) {
        fprintf(stderr, "mailune_parse_query did not return an ok record\n");
        return 1;
    }
    mailune_string_free(parsed);
    return 0;
}
