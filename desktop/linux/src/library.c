#include "library.h"

#include "mailune.h"

int mailune_linux_links(void) {
    char *answer = mailune_ready();
    if (answer == NULL) {
        return 0;
    }
    mailune_string_free(answer);
    return 1;
}
