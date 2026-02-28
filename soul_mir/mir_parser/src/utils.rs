pub(crate) struct EndBlock<T> {
    value: T,
    is_end: bool,
}
impl<T> EndBlock<T> {
    
    pub(crate) fn new(value: T, is_end: &bool) -> Self {
        Self { value, is_end: *is_end }
    }

    pub(crate) fn pass(self, is_end: &mut bool) -> T {
        *is_end = self.is_end;
        self.value
    }
}