#[derive(Debug, Clone, PartialEq)]
pub enum Number {
    Int(i64),
    Uint(u64),
    Float(f64),
}

#[derive(Debug, Clone)]
pub enum BinaryOperatorKind {
    Invalid,
    /// `+`
    Add, 
    /// `-`
    Sub, 
    /// `*`
    Mul, 
    /// `/`
    Div, 
    /// `log`
    Log, 
    /// `**`
    Pow, 
    /// `</`
    Root, 
    /// `%` 
    Mod, 

    /// `&`
    BitAnd, 
    /// `|`
    BitOr, 
    /// `^`
    BitXor, 
    
    /// `&&`
    LogAnd, 
    /// `||`
    LogOr, 
    /// `==`
    Eq, 
    /// `!=`
    NotEq, 
    /// `<`
    Lt, 
    /// `>`
    Gt, 
    /// `<=`
    Le, 
    /// `>=`
    Ge, 

    /// `..`
    Range, 
}

#[derive(Debug, Clone)]
pub enum UnaryOperatorKind {
    Invalid,
    /// `-`
    Neg, 
    /// `!`
    Not, 
    /// `++`
    Increment, 
    /// `--`
    Decrement, 
}