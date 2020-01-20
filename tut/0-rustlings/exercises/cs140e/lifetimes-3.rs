// FIXME: Make me compile! Diff budget: 2 lines.

// Do not modify the inner type &'a T.
struct RefWrapper<'a, T>(&'a T);

// Do not modify the inner type &'b RefWrapper<'a, T>.
struct RefWrapperWrapper<'b, 'a: 'b, T>(&'b RefWrapper<'a, T>);

pub fn main() {}
