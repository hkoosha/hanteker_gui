use druid::{Lens, LocalizedString};

pub(crate) mod f32_formatter;
pub(crate) mod label;
pub(crate) mod scope;
pub(crate) mod usize_formatter;

pub fn t<T>(text: &'static str) -> LocalizedString<T> {
    tt(text, text)
}

pub fn tt<T>(key: &'static str, text: impl Into<String>) -> LocalizedString<T> {
    let text: String = text.into();
    LocalizedString::new(key).with_placeholder(text)
}

struct LensAdapter<F0, F1> {
    pub f0: F0,
    pub f1: F1,
}

impl<U, T, F0, F1> Lens<U, T> for LensAdapter<F0, F1>
where
    F0: Fn(&U) -> T,
    F1: Fn(&mut U, T),
{
    fn with<V, F: FnOnce(&T) -> V>(&self, data: &U, f: F) -> V {
        let value = (self.f0)(data);
        f(&value)
    }

    fn with_mut<V, F: FnOnce(&mut T) -> V>(&self, data: &mut U, f: F) -> V {
        let mut value = (self.f0)(data);
        let result = f(&mut value);
        (self.f1)(data, value);
        result
    }
}

pub fn lens_of<U, T, F0, F1>(f0: F0, f1: F1) -> impl Lens<U, T>
where
    F0: Fn(&U) -> T,
    F1: Fn(&mut U, T),
{
    LensAdapter { f0, f1 }
}
