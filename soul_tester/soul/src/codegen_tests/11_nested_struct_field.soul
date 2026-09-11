// expect: 5
struct Inner {
    x: i32
}

struct Outer {
    inner: Inner
}

main(): i32 {
    inner: Inner = Inner{x: 1}
    mut o: Outer = Outer{inner: inner}
    o.inner.x = 5
    return o.inner.x
}
