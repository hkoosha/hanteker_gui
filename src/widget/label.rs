use druid::widget::Label;
use druid::{Data, UnitPoint, Widget, WidgetExt};

use crate::t;

pub fn label<T: Data>(text: &'static str) -> impl Widget<T> {
    Label::new(t(text)).align_horizontal(UnitPoint::LEFT)
}

pub fn label_c<T: Data>(text: &'static str) -> impl Widget<T> {
    Label::new(t(text)).align_horizontal(UnitPoint::CENTER)
}

pub fn label_ct<T: Data>(text: impl Into<String>) -> impl Widget<T> {
    let text: String = text.into();
    Label::new(text).align_horizontal(UnitPoint::CENTER)
}
