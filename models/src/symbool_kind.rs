/// Symbol kinds representing all possible symbols/tokens in the Soul language.
///
/// This enum covers operators, punctuation, brackets, and other symbols
/// that can appear in source code.
#[derive(Debug, Clone, PartialEq)]
pub enum SymboolKind {
    /// `+`
    Plus, 
    /// `++`
    DoublePlus,
    /// `-`
    Minus, 
    /// `--`
    DoubleMinus,
    /// `*`
    Star, 
    /// `/`
    Slash, 
    /// `**`
    DoubleStar, 
    /// `</`
    Root, 
    /// `%`
    Mod, 
    /// `&`
    And, 
    /// `@`
    ConstRef,
    /// `$`
    Money,
    /// `|`
    Or,
    /// `^`
    Xor,
    /// `&&`
    DoubleAnd, 
    /// `||`
    DoubleOr, 
    /// `=`
    Assign, 
    /// `:=`
    Declaration, 
    /// `+=`
    PlusEq,
    /// `-=`
    MinusEq,
    /// `*=`
    StarEq,
    /// `/=`
    SlashEq,
    /// `%=`
    ModEq,
    /// `&=`
    AndEq,
    /// `|=`
    OrEq,
    /// `^=`
    XorEq,
    /// `=>`
    LambdaArray,
    /// `==`
    Eq, 
    /// `!`
    Not,
    /// `?`
    Question,
    /// `??`
    DoubleQuestion,
    /// `!=`
    NotEq, 
    /// `<`
    LeftArray, 
    /// `>`
    RightArray, 
    /// `<=`
    Le, 
    /// `>=`
    Ge, 
    /// `:`
    Colon, 
    /// `::`
    DoubleColon, 
    /// `;`
    SemiColon, 
    /// `.`
    Dot, 
    /// `,`
    Comma,
    /// `..`
    DoubleDot,
    /// `[]`
    Array,
    /// `(`
    RoundOpen,
    /// `)`
    RoundClose,
    /// `[`
    SquareOpen,
    /// `]`
    SquareClose,
    /// `{`
    CurlyOpen,
    /// `}`
    CurlyClose,
}