#include "clib.h"

// ---------------- #Printers ----------------

void __csoul_printStr(cstr text) {
    printf("%s", text);
}

void __csoul_printChar(char ch) {
    putchar(ch);
}

// ---------------- #Formatters ----------------

str __csoul_fmtUint(uint num, u8 base, char buf[50]) {
    static cstr NUMBERS = "0123456789ABCDEF";
    str ptr = &buf[49];
    *ptr = '\0';

    do {
        *--ptr = NUMBERS[num % base];
        num /= base;
    } while(num);
    return ptr;
}

str __csoul_fmtInt(int num, u8 base, char buf[50]) {
    static cstr NUMBERS = "0123456789ABCDEF";
    str ptr = &buf[49];
    *ptr = '\0';

    int sign = 0;
    if (num < 0) {
        sign = 1;
        num = -num;
    }

    do {
        *--ptr = NUMBERS[num % base];
        num /= base;
    } while (num);

    if (sign) *--ptr = '-';
    return ptr;
}

static str __inner_fmt_uint(uint64_t n, u8 base, str buf, int max_digits) {
    static cstr NUMBERS = "0123456789ABCDEF";
    if (n == 0) {
        *buf++ = '0';
        return buf;
    }
    
    char temp[20];
    int i = 0;
    while (n > 0 && i < max_digits) {
        u8 digit = n % base;
        temp[i++] = NUMBERS[digit];
        n /= base;
    }
    
    while (i--) {
        *buf++ = temp[i];
    }
    return buf;
}

str __csoul_fmtFloat(double num, u8 base, char buf[50], u8 percision, bool capital) {
    static cstr NUMBERS = "0123456789abcdef";
    static cstr NUMBERS_LOWER = "0123456789abcdef";
    str start = buf;
    if (num < 0.0) {
        *buf++ = '-';
        num = -num;
    }

    uint whole = (uint)num;
    buf = __inner_fmt_uint(whole, base, buf, 20);
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