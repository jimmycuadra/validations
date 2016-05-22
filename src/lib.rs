//! Crate `validations` provides an interface to check the validity of arbitrary types.
//!
//! The `Validate` trait provides the `validate` method, which runs arbitrary validation logic and
//! returns a result indicating whether or not the value is valid. A return value of `Ok(())`
//! indicates a valid value. A return value of `Err(Errors)` indicates an invalid value,
//! and includes details of why the value failed validation.
//!
//! `Errors` is a container that can hold both general and field-specific validation
//! errors for an invalid value. An individual validation error is represented by
//! `Error`, which contains a human-readable error message, and an optional type of the
//! programmer's choice that includes additional contextual information about the error.
//!
//! Types that implement `Validate` should handle validation logic for each of their fields, as
//! necessary. If the type of a field implements `Validate` itself, it's also possible to delegate
//! to the field to validate itself and assign any resulting errors back to the parent type's
//! errors.
//!
//! Instead of implementing `Validate`, another approach is to implement validation logic inside the
//! constructor function of a type `T`, and return `Result<T, Errors>`, preventing an invalid value
//! from being created in the first place. This may not always be possible, as the value may be
//! created through other means. For example, the value may be deserialized from a format like JSON
//! from an external source. In this case, the `Validate` trait allows deserialization logic to be
//! decoupled from domain-level validation logic.
//!
//! # Examples
//!
//! Validating a value:
//!
//! ```
//! let entry = AddressBookEntry {
//!     cell_number: None,
//!     email: Some(Email("rcohle@dps.la.gov")),
//!     home_number: Some(PhoneNumber {
//!         area_code: "555",
//!         number: "555-5555",
//!     }),
//!     name: "Rust Cohle",
//! };
//!
//! assert!(entry.validate().is_ok());
//! ```
//!
//! Validating a value with a non-field-specific error:
//!
//! ```
//! let entry = AddressBookEntry {
//!     cell_number: None,
//!     email: Some(Email("rcohle@dps.la.gov")),
//!     home_number: None,
//!     name: "Rust Cohle",
//! };
//!
//! let errors = entry.validate().err().unwrap();
//!
//! assert_eq!(
//!     errors.base().unwrap()[0].message(),
//!     "at least one phone number is required".to_string()
//! );
//! ```
//!
//! Validating a value with a field error:
//!
//! ```
//! let entry = AddressBookEntry {
//!     cell_number: None,
//!     email: Some(Email("rcohle@dps.la.gov")),
//!     home_number: Some(PhoneNumber {
//!         area_code: "555",
//!         number: "555-5555",
//!     }),
//!     name: "",
//! };
//!
//! let errors = entry.validate().err().unwrap();
//!
//! assert_eq!(
//!     errors.field("name").unwrap().base().unwrap()[0].message(),
//!     "can't be blank".to_string()
//! );
//! ```

#![deny(missing_docs)]
#![deny(warnings)]

use std::any::Any;
use std::collections::hash_map::{Entry, HashMap};
use std::error::Error as StdError;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

/// An individual validation error.
#[derive(Debug)]
pub struct Error<T> where T: Debug + Any {
    details: Option<T>,
    message: String,
}

/// A collection of errors returned by a failed validation.
#[derive(Debug)]
pub struct Errors<T> where T: Debug + Any {
    base: Option<Vec<Error<T>>>,
    fields: Option<HashMap<String, Box<Errors<T>>>>,
}

/// An `Error` with no custom details, to avoid the generic parameter when not needed.
pub type SimpleError = Error<()>;

/// `Errors` with no custom details, to avoid the generic parameter when not needed.
pub type SimpleErrors = Errors<()>;

/// A validatable type.
pub trait Validate<T> where T: Debug + Any {
    /// Validates the value.
    ///
    /// If invalid, returns details about why the value failed validation.
    fn validate(&self) -> Result<(), Errors<T>>;
}

impl<T> Error<T> where T: Debug + Any {
    /// Constructs a validation error.
    pub fn new<S>(message: S) -> Self where S: Into<String> {
        Error {
            details: None,
            message: message.into(),
        }
    }

    /// Constructs a validation error with additional details.
    pub fn with_details<S>(message: S, details: T) -> Self where S: Into<String> {
        Error {
            details: Some(details),
            message: message.into(),
        }
    }

    /// Additional contextual information about the error, if provided.
    pub fn details(&self) -> Option<&T> {
        self.details.as_ref()
    }

    /// Sets the details of this error.
    pub fn set_details(&mut self, details: T) {
        self.details = Some(details);
    }

    /// A human-readable message explaining the error.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl<T> Display for Error<T> where T: Debug + Any {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", &self.message)
    }
}

impl<T> StdError for Error<T> where T: Debug + Any {
    fn description(&self) -> &str {
        &self.message
    }
}

impl<T> Errors<T> where T: Debug + Any {
    /// Constructs an empty `Errors` value.
    pub fn new() -> Self {
        Errors {
            base: None,
            fields: None,
        }
    }

    /// Adds a validation error that is not specific to any field.
    pub fn add_error(&mut self, error: Error<T>) {
        match self.base {
            Some(ref mut base_errors) => base_errors.push(error),
            None => self.base = Some(vec![error]),
        }
    }

    /// Adds a validation error for the given field.
    ///
    /// Calling this method will overwrite any errors assigned via `set_field_errors`.
    pub fn add_field_error<S>(&mut self, field: S, error: Error<T>) where S: Into<String>{
        match self.fields {
            Some(ref mut field_errors) => {
                match field_errors.entry(field.into()) {
                    Entry::Occupied(mut entry) => {
                        entry.get_mut().add_error(error);
                    }
                    Entry::Vacant(entry) => {
                        let mut errors = Errors::new();

                        errors.add_error(error);

                        entry.insert(Box::new(errors));
                    }
                }
            }
            None => {
                let mut errors = Errors::new();

                errors.add_error(error);

                let mut map = HashMap::new();

                map.insert(field.into(), Box::new(errors));

                self.fields = Some(map);
            }
        }
    }

    /// A slice of non-field-specific errors, if any.
    pub fn base<'a>(&'a self) -> Option<&'a [Error<T>]> {
        self.base.as_ref().map(Vec::as_slice)
    }

    /// The `Errors` for the given field, if any.
    pub fn field<F>(&self, field: F) -> Option<&Box<Errors<T>>> where F: Into<String> {
        if self.fields.is_some() {
            self.fields.as_ref().unwrap().get(&field.into())
        } else {
            None
        }
    }

    /// Returns `true` if there are no errors.
    pub fn is_empty(&self) -> bool {
        self.base.is_none() && self.fields.is_none()
    }

    /// Sets the given field's errors to the given `Errors`.
    ///
    /// This is useful if the field itself implements `Validate`. In that case, the parent type can
    /// simply delegate to the field to validate itself and assign the resulting errors using this
    /// method.
    ///
    /// Calling this method will overwrite any field errors previously added with
    /// `add_field_error`.
    pub fn set_field_errors<S>(&mut self, field: S, errors: Errors<T>) where S: Into<String>{
        match self.fields {
            Some(ref mut field_errors) => {
                field_errors.insert(field.into(), Box::new(errors));
            }
            None => {
                let mut map = HashMap::new();

                map.insert(field.into(), Box::new(errors));

                self.fields = Some(map);
            }
        }
    }
}

impl<T> Display for Errors<T> where T: Debug + Any {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "validation failed")
    }
}

impl<T> StdError for Errors<T> where T: Debug + Any {
    fn description(&self) -> &str {
        "validation failed"
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, Errors, Validate};

    #[derive(Debug)]
    struct AddressBookEntry {
        cell_number: Option<PhoneNumber>,
        email: Option<Email>,
        home_number: Option<PhoneNumber>,
        name: &'static str,
    }

    #[derive(Debug)]
    struct Email(&'static str);

    #[derive(Debug)]
    struct PhoneNumber {
        area_code: &'static str,
        number: &'static str,
    }

    #[derive(Debug)]
    struct InvalidCharacters {
        invalid_characters: Vec<char>,
    }


    impl Validate<InvalidCharacters> for AddressBookEntry {
        fn validate(&self) -> Result<(), Errors<InvalidCharacters>> {
            let mut errors = Errors::new();

            if self.cell_number.is_none() && self.home_number.is_none() {
                errors.add_error(Error::new("at least one phone number is required"));
            }

            if self.name.len() == 0 {
                errors.add_field_error("name", Error::new("can't be blank"));
            }

            if let Some(ref email) = self.email {
                if let Err(field_errors) = email.validate() {
                    errors.set_field_errors("email", field_errors);
                }
            }

            let numbers_to_check = [
                ("home_number", &self.home_number),
                ("cell_number", &self.cell_number),
            ];

            for &(field_name, field) in &numbers_to_check {
                if field.is_some() {
                    let invalid_characters = InvalidCharacters::check_digits(
                        &field.as_ref().unwrap().full_number()
                    );

                    if invalid_characters.len() > 0 {
                        errors.add_field_error(
                            field_name,
                            Error::with_details(
                                "has invalid characters",
                                InvalidCharacters {
                                    invalid_characters: invalid_characters,
                                },
                            ),
                        );
                    }
                }
            }

            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        }
    }

    impl Validate<InvalidCharacters> for Email {
        fn validate(&self) -> Result<(), Errors<InvalidCharacters>> {
            let email = self.0;

            if !email.contains("@") {
                let mut errors = Errors::new();

                errors.add_error(Error::new("must contain an @ symbol"));

                return Err(errors);
            }

            Ok(())
        }
    }

    impl PhoneNumber {
        pub fn full_number(&self) -> String {
            format!("{}-{}", self.area_code, self.number)
        }
    }

    impl InvalidCharacters {
        pub fn check_digits(number: &str) -> Vec<char> {
            number.replace("-", "").chars().filter(|c| !c.is_digit(10)).collect()
        }

        pub fn invalid_characters(&self) -> &[char] {
            self.invalid_characters.as_slice()
        }
    }

    #[test]
    fn valid_value() {
        let entry = AddressBookEntry {
            cell_number: None,
            email: Some(Email("rcohle@dps.la.gov")),
            home_number: Some(PhoneNumber {
                area_code: "555",
                number: "555-5555",
            }),
            name: "Rust Cohle",
        };

        assert!(entry.validate().is_ok());
    }

    #[test]
    fn base_error() {
        let entry = AddressBookEntry {
            cell_number: None,
            email: Some(Email("rcohle@dps.la.gov")),
            home_number: None,
            name: "Rust Cohle",
        };

        let errors = entry.validate().err().unwrap();

        assert_eq!(
            errors.base().unwrap()[0].message(),
            "at least one phone number is required".to_string()
        );
    }

    #[test]
    fn field_error() {
        let entry = AddressBookEntry {
            cell_number: None,
            email: Some(Email("rcohle@dps.la.gov")),
            home_number: Some(PhoneNumber {
                area_code: "555",
                number: "555-5555",
            }),
            name: "",
        };

        let errors = entry.validate().err().unwrap();

        assert_eq!(
            errors.field("name").unwrap().base().unwrap()[0].message(),
            "can't be blank".to_string()
        );
    }

    #[test]
    fn delegate_to_field() {
        let entry = AddressBookEntry {
            cell_number: None,
            email: Some(Email("rcohle")),
            home_number: Some(PhoneNumber {
                area_code: "555",
                number: "555-5555",
            }),
            name: "Rust Cohle",
        };

        let errors = entry.validate().err().unwrap();

        assert_eq!(
            errors.field("email").unwrap().base().unwrap()[0].message(),
            "must contain an @ symbol".to_string()
        );
    }

    #[test]
    fn details() {
        let entry = AddressBookEntry {
            cell_number: None,
            email: Some(Email("rcohle@dps.la.gov")),
            home_number: Some(PhoneNumber {
                area_code: "555",
                number: "x55-55t5",
            }),
            name: "",
        };

        let errors = entry.validate().err().unwrap();

        let invalid_characters = errors.field("home_number").unwrap().base().unwrap()[0]
                .details().unwrap().invalid_characters();

        assert!(invalid_characters.contains(&'x'));
        assert!(invalid_characters.contains(&'t'));
    }
}
