use druid::text::{Formatter, Selection, Validation, ValidationError};
use druid::widget::TextBox;
use druid::Widget;
use thiserror::Error;

pub struct UsizeFormatter {
    pub min: Option<usize>,
    pub max: Option<usize>,
}

impl Formatter<usize> for UsizeFormatter {
    fn format(&self, value: &usize) -> String {
        value.to_string()
    }

    fn validate_partial_input(&self, input: &str, _: &Selection) -> Validation {
        if input.is_empty() {
            Validation::success()
        } else if let Ok(v) = input.parse::<usize>() {
            validate_usize(v, self.min, self.min)
        } else {
            UsizeValidationError::BadCharacter.into()
        }
    }

    fn value(&self, input: &str) -> Result<usize, ValidationError> {
        if let Ok(v) = input.parse::<usize>() {
            let validation = validate_usize(v, self.min, self.min);
            if validation.is_err() {
                Err(validation.error().unwrap().clone())
            } else {
                Ok(v)
            }
        } else {
            let validation: Validation = UsizeValidationError::BadCharacter.into();
            Err(validation.error().unwrap().clone())
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<Validation> for UsizeValidationError {
    fn into(self) -> Validation {
        Validation::failure(ValidationError::new(self))
    }
}

#[derive(Debug, Clone, Error)]
enum UsizeValidationError {
    #[error("bad character")]
    BadCharacter,
    #[error("value too low")]
    MinViolation,
    #[error("value too high")]
    MaxViolation,
}

fn validate_usize(v: usize, min: Option<usize>, max: Option<usize>) -> Validation {
    if min.is_some() && v < min.unwrap() {
        UsizeValidationError::MinViolation.into()
    } else if max.is_some() && v > max.unwrap() {
        UsizeValidationError::MaxViolation.into()
    } else {
        Validation::success()
    }
}

pub fn usize_text(min: Option<usize>, max: Option<usize>) -> impl Widget<usize> {
    TextBox::new().with_formatter(UsizeFormatter { min, max })
}

pub fn usize_text_unrestricted() -> impl Widget<usize> {
    usize_text(None, None)
}
