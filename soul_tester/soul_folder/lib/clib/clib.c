#include "clib.h"

// ---------------- #Array ----------------      

SoulArray __clib_mallocString(uint len) {
    void* ptr = malloc(len);
    if(ptr == NULL)
        len = 0;
    return (SoulArray){.ptr = ptr, .len = len};
}

// ---------------- #File ----------------      

i64 __clib_fileLen(FILE* file) {
    if (fseek(file, 0, SEEK_END) != 0) {
        fclose(file);
        return -1;
    }
    i64 len = ftell(file);
    fseek(file, 0, SEEK_SET);
    return len;
}

i64 __clib_fileToStr(FILE* file, uint fileLen, /*out*/SoulStr* str) {
    fseek(file, 0, SEEK_SET);
    if(fileLen < 0 || str->len+1 < fileLen) 
        return -1;

    return fread(str->ptr, 1, fileLen, file);
}

bool __clib_filePrint(FILE* file) {
    i64 len = __clib_fileLen(file);
    if (len < 0) {
        return false;
    }

    char* buffer = malloc(len);
    fread(buffer, 1, len, file);
    printf("%*s", len, buffer);
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
    for(int i = 0; i < len; i++) {
        putchar(ptr[i]);
    }
}

void __clib_printCStr(const str text) {
    printf("%s", text);
}

void __clib_printChar(char ch) {
    putchar(ch);
}

// ---------------- #Formatters ----------------

str __clib_fmtUint(uint num, u8 base, char buf[50], bool capital) {
    const str NUMBERS = "0123456789ABCDEF";
    const str NUMBERS_LOWER = "0123456789abcdef";
    str ptr = &buf[49];
    *ptr = '\0';
    
    if (base < 2 || base > 16) {
        return NULL;
    }

    const str numbers = capital ? NUMBERS : NUMBERS_LOWER;
    do {
        *--ptr = numbers[num % base];
        num /= base;
    } while(num);
    return ptr;
}

str __clib_fmtInt(int num, u8 base, char buf[50], bool capital) {
    const str NUMBERS = "0123456789ABCDEF";
    const str NUMBERS_LOWER = "0123456789abcdef";

    str ptr = &buf[49];
    *ptr = '\0';

    if (base < 2 || base > 16) {
        return NULL;
    }
    
    int sign = 0;
    if (num < 0) {
        sign = 1;
        num = -num;
    }

    const str numbers = capital ? NUMBERS : NUMBERS_LOWER;
    do {
        *--ptr = numbers[num % base];
        num /= base;
    } while (num);

    if (sign) *--ptr = '-';
    return ptr;
}

static str __inner_fmt_uint(uint64_t n, u8 base, str buf, int max_digits, bool capital) {
    const str NUMBERS = "0123456789ABCDEF";
    const str NUMBERS_LOWER = "0123456789abcdef";

    if (n == 0) {
        *buf++ = '0';
        return buf;
    }
    if (base < 2 || base > 16) {
        return NULL;
    }

    const str numbers = capital ? NUMBERS : NUMBERS_LOWER;
    char temp[20];
    int i = 0;
    while (n > 0 && i < max_digits) {
        u8 digit = n % base;
        temp[i++] = numbers[digit];
        n /= base;
    }
    
    while (i--) {
        *buf++ = temp[i];
    }
    return buf;
}

str __clib_fmtFloat(double num, u8 base, char buf[50], u8 percision, bool capital) {
    const str NUMBERS = "0123456789ABCDEF";
    const str NUMBERS_LOWER = "0123456789abcdef";

    str start = buf;
    if (num < 0.0) {
        *buf++ = '-';
        num = -num;
    }
    if (base < 2 || base > 16) {
        return NULL;
    }

    uint whole = (uint)num;
    buf = __inner_fmt_uint(whole, base, buf, 20, capital);
    if(percision == 0) {
        *buf = '\0';
        return buf;
    }

    const str numbers = capital ? NUMBERS : NUMBERS_LOWER;
    *buf++ = '.';
    double frac = num - (double)whole;
    for(int i = 0; i < percision; i++) {
        frac *= base;
        u64 digit = (u64)frac;
        *buf++ = numbers[digit];
        frac -= (double)digit;
    }

    *buf = '\0';
    return start;
}

// ---------------- #Time ----------------

void __clib_delay_sec(int seconds) {
    clock_t start = clock();
    while (((double)(clock() - start)) / CLOCKS_PER_SEC < seconds) {}
}

Duration __clib_Duration_now() {
    struct timespec ts = {0};

#ifdef _WIN32
    const u64 NANO_PER_TICK = 100ULL;      
    const u64 WINDOWS_TICK = 10000000ULL;
    const u64 EPOCH_DIFF = 11644473600ULL;
    ULARGE_INTEGER ft;
    GetSystemTimePreciseAsFileTime((LPFILETIME)&ft);
    ts.tv_sec = (long)(ft.QuadPart / WINDOWS_TICK - EPOCH_DIFF);
    ts.tv_nsec = (long)((ft.QuadPart % WINDOWS_TICK) * NANO_PER_TICK);
#elif defined(__unix__) || defined(__APPLE__)
    clock_gettime(CLOCK_REALTIME, &ts);
#else
    // Fallback: second precision only
    time_t t = time(NULL);
    ts.tv_sec = (long)t;
    ts.tv_nsec = 0;
#endif

    return (Duration){
        .sec = (u64)ts.tv_sec,
        .nano = (u32)ts.tv_nsec, 
    };
}

// ---------------- #Pointers ----------------

void* __clib_Nullptr() {return NULL;}

u8* __clib_offset(u8* ptr, size_t index) {
    return ptr + index;
}