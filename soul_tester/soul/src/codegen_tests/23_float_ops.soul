// expect: 1
main(): i32 {
    a: f64 = 10.0
    b: f64 = 4.0

    sum: f64 = a + b
    diff: f64 = a - b
    prod: f64 = a * b
    quot: f64 = a / b
    rem: f64 = a % b

    mut ok: bool = sum == 14.0
    ok = ok && diff == 6.0
    ok = ok && prod == 40.0
    ok = ok && quot == 2.5
    ok = ok && rem == 2.0
    ok = ok && a > b
    ok = ok && b < a
    ok = ok && a >= 10.0
    ok = ok && b <= 4.0
    ok = ok && a != b

    if ok {
        return 1
    }
    return 0
}
