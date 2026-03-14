#ifndef __MAIN_H__
#define __MAIN_H__

#include <stdio.h>
#include <stdlib.h>

void* __csoul_alloc(int size) {
    return malloc(size);
}

void __csoul_freeNonNull(void* ptr) {
    free(ptr);
}

void __csoul_printChar(char ch) {
    putchar(ch);
}

void __csoul_printInt(int value) {
    printf("%d", value);
}
#endif