pub trait SyntaxDisplay {
    fn display(&self) -> String;
    fn inner_display(&self, sb: &mut String, tab: usize, is_last: bool);
}

pub fn tree_prefix(tab: usize, is_last: bool) -> String {
    let mut sb = String::new(); 
    if tab == 0 {
        return sb
    }

    for _ in 0..tab - 1 {
        sb.push_str("│   ");
    }

    sb.push_str(if is_last { "└── " } else { "├── " });
    sb
}