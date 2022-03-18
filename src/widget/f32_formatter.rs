use druid::text::{Formatter, Selection, Validation, ValidationError};
use druid::Widget;
use druid::widget::TextBox;
use log::info;
use thiserror::Error;

pub struct FloatFormatter {
    pub min: Option<f32>,
    pub max: Option<f32>,
}

impl Formatter<f32> for FloatFormatter {
    fn format(&self, value: &f32) -> String {
        value.to_string()
    }

    fn validate_partial_input(&self, input: &str, _: &Selection) -> Validation {
        if input.is_empty()
            || input.len() == 1 && (input.starts_with('+') || input.starts_with('-'))
        {
            Validation::success()
        } else if let Ok(v) = input.parse::<f32>() {
            validate_f32(v, self.min, self.min)
        } else {
            FloatValidationError::BadCharacter.into()
        }
    }

    fn value(&self, input: &str) -> Result<f32, ValidationError> {
        if let Ok(v) = input.parse::<f32>() {
            let validation = validate_f32(v, self.min, self.min);
            if validation.is_err() {
                Err(validation.error().unwrap().clone())
            } else {
                Ok(v)
            }
        } else {
            let validation: Validation = FloatValidationError::BadCharacter.into();
            Err(validation.error().unwrap().clone())
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<Validation> for FloatValidationError {
    fn into(self) -> Validation {
        Validation::failure(ValidationError::new(self))
    }
}

#[derive(Debug, Clone, Error)]
enum FloatValidationError {
    #[error("bad character")]
    BadCharacter,
    #[error("bad value")]
    ValueViolation,
    #[error("value too low")]
    MinViolation,
    #[error("value too high")]
    MaxViolation,
}

fn validate_f32(v: f32, min: Option<f32>, max: Option<f32>) -> Validation {
    if v.is_nan() {
        FloatValidationError::ValueViolation.into()
    } else if v.is_infinite() && v.is_sign_negative() || min.is_some() && !min.unwrap().is_infinite() && v < min.unwrap() {
        FloatValidationError::MinViolation.into()
    } else if v.is_infinite() && v.is_sign_positive() || max.is_some() && !max.unwrap().is_infinite() && v > max.unwrap() {
        FloatValidationError::MaxViolation.into()
    } else {
        Validation::success()
    }
}

pub fn float_text(min: Option<f32>, max: Option<f32>) -> impl Widget<f32> {
    TextBox::new().with_formatter(FloatFormatter { min, max })
}

pub fn float_text_unrestricted() -> impl Widget<f32> {
    float_text(None, None)
}
