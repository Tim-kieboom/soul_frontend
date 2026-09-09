answer(): int {
    return 42
}

add(a: int, b: int): int {
    c := a + b
    return c
}

computeChain(a: int, b: int, c: int): int {
    x := a + b
    y := x * c
    z := y - a
    return z
}

weightedSum(a: int, b: int, c: int): int {
    return a + b * c
}

arithmeticOps(a: int, b: int): int {
    CONST :: 1
    sum := a + b
    diff := a - b
    product := a * b
    quotient := a / b
    remainder := a % b
    return sum + diff + product + quotient - remainder
}

addUint(a: uint, b: uint): uint {
    return a + b
}

average(mut a: f64, b: f64): f64 {
    sum := a + b
    return sum
}

identity(flag: bool): bool {
    return flag
}

alwaysTrue(): bool {
    return true
}

ifWithoutElse(): int {
    if true {
        return 1
    }
    return 2
}

ifElse(): int {
    if true {
        return 1
    } else {
        return 2
    }
}

ifElseIfElse(): int {
    if false {
        return 1
    } else if true {
        return 2
    } else {
        return 3
    }
}

loopWithBreak(): int {
    for true {
        break
    }
    return 1
}

loopWithContinue(): int {
    for true {
        continue
    }
    return 1
}

loopWithNestedIfBreak(): int {
    for true {
        if true {
            break
        }
    }
    return 1
}

comparisonCondition(a: int, b: int): int {
    if a > b {
        return 1
    }
    return 2
}

logicalAndCondition(a: int, b: int, c: int, d: int): int {
    if a > b && c < d {
        return 1
    }
    return 2
}

logicalOrCondition(a: int, b: int): bool {
    return a == b || a > b
}

whileWithComparisonCondition(mut n: int): int {
    for n > 0 {
        n -= 1
    }
    return n
}

boolVariableCondition(flag: bool): int {
    if flag {
        return 1
    }
    return 2
}

notCondition(flag: bool): int {
    if !flag {
        return 1
    }
    return 2
}

notOfComparisonCondition(a: int, b: int): int {
    if !(a > b) {
        return 1
    }
    return 2
}

reassignParameter(mut a: int): int {
    a = 5
    return a
}

reassignLocal(): int {
    mut x := 1
    x = 2
    return x
}

callee(x: int): int {
    return x + 1
}

callResultReturned(): int {
    return callee(41)
}

callEmbeddedInExpression(a: int): int {
    return a + callee(a)
}

callWithMultipleArguments(x: int, y: int): int {
    return x + y
}

callWithExpressionArgument(a: int, b: int): int {
    return callWithMultipleArguments(a, b + 1)
}

logMessage(): none {
}

callDiscardingNonNoneResult(): none {
    callee(1);
}

callingANoneFunction(): none {
    logMessage()
}

assertSimpleCondition(): none {
    assert(true)
}

assertComparisonCondition(a: int, b: int): none {
    assert(a > b)
}

panicUnconditionally(): none {
    panic("oops")
}
