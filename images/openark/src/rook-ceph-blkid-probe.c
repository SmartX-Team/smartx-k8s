#include <blkid/blkid.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdlib.h>

int main(int argc, char **argv) {
    char *end;
    char *cursor;
    long value;
    blkid_probe probe;
    int result;

    if (argc != 2 || argv[1][0] == '-' || argv[1][0] == '\0')
        return 255;
    for (cursor = argv[1]; *cursor; ++cursor)
        if (*cursor < '0' || *cursor > '9')
            return 255;
    errno = 0;
    value = strtol(argv[1], &end, 10);
    if (errno || *end || value > INT_MAX || fcntl((int)value, F_GETFD) == -1)
        return 255;
    probe = blkid_new_probe();
    if (!probe)
        return 255;
    result = blkid_probe_set_device(probe, (int)value, 0, 0);
    if (!result)
        result = blkid_do_safeprobe(probe);
    blkid_free_probe(probe);
    if (result == -2)
        return 254;
    if (result == -1 || (result != 0 && result != 1))
        return 255;
    return result;
}
