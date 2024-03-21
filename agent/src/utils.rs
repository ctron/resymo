pub(crate) fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    value == &Default::default()
}
