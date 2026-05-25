#include "clib.h"

// ---------------- #Array ----------------      

u8* __clib_bytesRealloc(uint len, u8* ptr) {
    if(ptr == NULL)
        return malloc(len);
    
    u8* new = realloc(ptr, len);
    if(ptr != NULL) 
        free(ptr);

    return new;
}

/// !!DO NOT REMOVE!! this is a internal used function for array compare
bool __clib_arrayEqual(u32 elementSize, u8* leftPtr, uint leftSize, u8* rightPtr, uint rightSize) {
    if(leftSize != rightSize) {
        return false;
    }

    return memcmp(leftPtr, rightPtr, leftSize * elementSize) == 0;
}

// ---------------- #File ----------------      

i64 __clib_fileLen(FILE* file) {
    if (fseek(file, 0L, SEEK_END) != 0) return -1;
    i64 size = ftell(file);
    rewind(file);
    return size;
}

bool __clib_filePrint(FILE* file) {
    i64 len = __clib_fileLen(file);
    if (len < 0)
        return false;

    char* buffer = malloc(len);
    if(!buffer)
        return false;

    fread(buffer, 1, len, file);
    printf("%*s", (int)len, buffer);
    return true;
}

// ---------------- #Math ----------------      

#define __POW(ty) ty __clib_pow_##ty(ty a, ty b) { \
    return (ty)pow((f64)a, (f64)b);                \
}

#define __ROOT(ty) ty __clib_root_##ty(ty exp, ty base) { \
    return (ty)pow((f64)base, (f64)1.0 / (f64)exp);       \
}

/// impl define marcro for all number types
#define __IMPL_NUMBERS(impl)    \
    impl(i8);impl(i16);         \
    impl(i32);impl(i64);        \
    impl(u8);impl(u16);         \
    impl(u32);impl(u64);        \
    impl(f32);impl(f64);

__IMPL_NUMBERS(__POW);
__IMPL_NUMBERS(__ROOT);

double __clib_log10(double base) {
    return log10(base);
}

double __clib_log2(double base) {
    return log2(base);
}

double __clib_log(double exp, double base) {
    return log10(exp) / log10(base);
}

// ---------------- #Printers ----------------

void __clib_printSoulStr(const char* ptr, const uint len) {
    printf("%*s", (int)len, ptr);
}

void __clib_printCStr(const str text) {
    printf("%s", text);
}

void __clib_printChar(const char ch) {
    putchar(ch);
}

void __clib_printI64(const i64 n) {
    printf("%lld", n);
}

void __clib_printU64(const u64 n) {
    printf("%llu", n);
}

void __clib_printF64(const f64 n) {
    printf("%f", n);
}

#define FMT_I64_BUFFER_SIZE 32
int __clib_fmtI64(const i64 n, /*out*/char buf[FMT_I64_BUFFER_SIZE]) {
    return snprintf(buf, FMT_I64_BUFFER_SIZE, "%lld", n);
}

#define FMT_U64_BUFFER_SIZE 25
int  __clib_fmtU64(const u64 n, /*out*/char buf[FMT_U64_BUFFER_SIZE]) {
    return snprintf(buf, FMT_U64_BUFFER_SIZE, "%llu", n);
}

#define FMT_F64_BUFFER_SIZE 64
int __clib_fmtF64(const f64 n, /*out*/char buf[FMT_F64_BUFFER_SIZE]) {
    return snprintf(buf, FMT_F64_BUFFER_SIZE, "%f", n);
}

// ---------------- #Time ----------------

static inline Duration Duration_Init(u64 seconds, u32 nanoSeconds) {
    return (Duration){
        .sec = seconds, 
        .nano = nanoSeconds,
    };
}

void __clib_delay_sec(int seconds) {
    clock_t start = clock();
    while (((double)(clock() - start)) / CLOCKS_PER_SEC < seconds) {}
}

void __clib_Duration_now(/*out*/Duration* duration) {
#ifdef _WIN32
    const u64 NANO_PER_TICK = 100ULL;      
    const u64 WINDOWS_TICK = 10000000ULL;
    const u64 EPOCH_DIFF = 11644473600ULL;
    ULARGE_INTEGER ft;
    GetSystemTimePreciseAsFileTime((LPFILETIME)&ft);
    duration->sec = ft.QuadPart / WINDOWS_TICK - EPOCH_DIFF;
    duration->nano = (ft.QuadPart % WINDOWS_TICK) * NANO_PER_TICK;
#elif defined(__unix__) || defined(__APPLE__)
    struct timespec ts = {0};
    clock_gettime(CLOCK_REALTIME, &ts);
    duration->sec = ts.tv_sec;
    duration->nano = ts.tv_nsec;
#else
    // Fallback: second precision only
    time_t t = time(NULL);
    duration->sec = (long)t;
    duration->nano = 0;
#endif
}