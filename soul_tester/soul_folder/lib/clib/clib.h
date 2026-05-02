#ifndef __CLIB_H__
#define __CLIB_H__
#include <time.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <stdbool.h>
#ifdef _WIN32
#include <WTypesbase.h>
#endif

/* 
to generate/run exe run commands:

    clang -c .\soul_folder\lib\clib\clib.c -o .\soul_folder\output\clib.o
    clang .\soul_folder\output\out.ll .\soul_folder\output\clib.o -o .\soul_folder\output\out.exe
    clang -S .\soul_folder\output\out.ll .\soul_folder\output\clib.o -o .\soul_folder\output\out.s
    .\soul_folder\output\out.exe
*/

typedef int8_t i8;
typedef int16_t i16;
typedef int32_t i32;
typedef int64_t i64;

typedef uint8_t u8;
typedef uint16_t u16;
typedef uint32_t u32;
typedef uint64_t u64;
typedef size_t uint;

typedef float f32;
typedef double f64;

typedef char* str;

typedef struct {
    u64 sec;
    u32 nano;
} Duration;

typedef struct {
    char* ptr;
    uint len;
} SoulStr;

#endif