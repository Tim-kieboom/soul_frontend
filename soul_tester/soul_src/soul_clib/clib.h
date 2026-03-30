#ifndef __BETTERINTS_H__
#define __BETTERINTS_H__
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

    clang -c .\soul_src\soul_clib\clib.c -o .\output\clib.o
    clang .\output\out.ll .\output\clib.o -o .\output\out.exe
    .\output\out.exe
*/

typedef int8_t i8;
typedef int16_t i16;
typedef int32_t i32;
typedef int64_t i64;

typedef uint8_t u8;
typedef uint16_t u16;
typedef uint32_t u32;
typedef uint64_t u64;
typedef unsigned int uint;

typedef float f32;
typedef double f64;

typedef char* str;

typedef struct {
    u64 sec;
    u32 nano;
} Duration;

#endif