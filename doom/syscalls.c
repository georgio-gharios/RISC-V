// The POSIX-ish layer picolibc expects, implemented on rv-doom's devices.
//   stdout/stderr -> UART
//   one read-only file "doom1.wad", served from memory (embedded by wad.S)
//   everything else (writes, mkdir, rename, ...) fails with -1
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>

#define UART ((volatile unsigned char *)0x10000000)

extern const unsigned char wad_data[];
extern const unsigned int wad_size;

// ---- stdio streams ----
static int uart_put(char c, FILE *f) {
    (void)f;
    UART[0] = c;
    return (unsigned char)c;
}
static FILE uart_file = FDEV_SETUP_STREAM(uart_put, NULL, NULL, _FDEV_SETUP_WRITE);
FILE *const stdin = &uart_file;
FILE *const stdout = &uart_file;
FILE *const stderr = &uart_file;

// ---- in-memory WAD file ----
#define WAD_FD 3
static long wad_pos;
static int wad_open;

static int is_wad(const char *path) {
    const char *slash = strrchr(path, '/');
    return strcmp(slash ? slash + 1 : path, "doom1.wad") == 0;
}

int open(const char *path, int flags, ...) {
    if (is_wad(path) && (flags & 3) == 0 /* O_RDONLY */ && !wad_open) {
        wad_open = 1;
        wad_pos = 0;
        return WAD_FD;
    }
    errno = ENOENT;
    return -1;
}

ssize_t read(int fd, void *buf, size_t n) {
    if (fd != WAD_FD || !wad_open) { errno = EBADF; return -1; }
    if (wad_pos >= (long)wad_size) return 0;
    if (n > wad_size - wad_pos) n = wad_size - wad_pos;
    memcpy(buf, wad_data + wad_pos, n);
    wad_pos += n;
    return n;
}

off_t lseek(int fd, off_t offset, int whence) {
    if (fd != WAD_FD || !wad_open) { errno = EBADF; return -1; }
    long base = whence == SEEK_SET ? 0 : whence == SEEK_CUR ? wad_pos : (long)wad_size;
    if (base + offset < 0) { errno = EINVAL; return -1; }
    wad_pos = base + offset;
    return wad_pos;
}

int close(int fd) {
    if (fd == WAD_FD && wad_open) { wad_open = 0; return 0; }
    errno = EBADF;
    return -1;
}

ssize_t write(int fd, const void *buf, size_t n) {
    if (fd != 1 && fd != 2) { errno = EBADF; return -1; }
    for (size_t i = 0; i < n; i++) UART[0] = ((const char *)buf)[i];
    return n;
}

int fstat(int fd, struct stat *st) {
    memset(st, 0, sizeof *st);
    if (fd == WAD_FD && wad_open) { st->st_mode = S_IFREG; st->st_size = wad_size; return 0; }
    errno = EBADF;
    return -1;
}

int stat(const char *path, struct stat *st) {
    (void)st;
    errno = ENOENT;
    return is_wad(path) ? 0 : -1;
}

int unlink(const char *path) { (void)path; errno = EROFS; return -1; }
int mkdir(const char *path, mode_t mode) { (void)path; (void)mode; errno = EROFS; return -1; }
int rename(const char *from, const char *to) { (void)from; (void)to; errno = EROFS; return -1; }
int system(const char *cmd) { (void)cmd; return -1; }

// exit() ends here: halt the emulator with the exit status in a0.
void _exit(int status) {
    register int a0 __asm__("a0") = status;
    __asm__ volatile("ecall" : : "r"(a0));
    for (;;) {}
}
