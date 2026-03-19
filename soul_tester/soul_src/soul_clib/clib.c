#include "clib.h"

// ---------------- #Printers ----------------

void __clib_printStr(cstr text) {
    printf("%s", text);
}

void __clib_printChar(char ch) {
    putchar(ch);
}

// ---------------- #Formatters ----------------

str __clib_fmtUint(uint num, u8 base, char buf[50], bool capital) {
    static cstr NUMBERS = "0123456789ABCDEF";
    static cstr NUMBERS_LOWER = "0123456789abcdef";
    str ptr = &buf[49];
    *ptr = '\0';

    cstr numbers = capital ? NUMBERS : NUMBERS_LOWER;
    do {
        *--ptr = numbers[num % base];
        num /= base;
    } while(num);
    return ptr;
}

str __clib_fmtInt(int num, u8 base, char buf[50], bool capital) {
    static cstr NUMBERS = "0123456789ABCDEF";
    static cstr NUMBERS_LOWER = "0123456789abcdef";
    str ptr = &buf[49];
    *ptr = '\0';

    int sign = 0;
    if (num < 0) {
        sign = 1;
        num = -num;
    }

    cstr numbers = capital ? NUMBERS : NUMBERS_LOWER;
    do {
        *--ptr = numbers[num % base];
        num /= base;
    } while (num);

    if (sign) *--ptr = '-';
    return ptr;
}

static str __inner_fmt_uint(uint64_t n, u8 base, str buf, int max_digits, bool capital) {
    static cstr NUMBERS = "0123456789ABCDEF";
    static cstr NUMBERS_LOWER = "0123456789abcdef";
    if (n == 0) {
        *buf++ = '0';
        return buf;
    }
    
    cstr numbers = capital ? NUMBERS : NUMBERS_LOWER;
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
    static cstr NUMBERS = "0123456789ABCDEF";
    static cstr NUMBERS_LOWER = "0123456789abcdef";
    str start = buf;
    if (num < 0.0) {
        *buf++ = '-';
        num = -num;
    }

    uint whole = (uint)num;
    buf = __inner_fmt_uint(whole, base, buf, 20, capital);
    if(percision == 0) {
        *buf = '\0';
        return buf;
    }

    cstr numbers = capital ? NUMBERS : NUMBERS_LOWER;
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

void __clib_delay_ms(i64 ms) {
    clock_t start = clock();
    clock_t wait = ms * (CLOCKS_PER_SEC / 1000);
    while ((clock() - start) < wait) {
        /*no-op*/;
    }
}

u64 __clib_clock() {
    return (u64)clock();
}

u64 __clib_CLOCKS_PER_SEC() {
    return CLOCKS_PER_SEC;
}

