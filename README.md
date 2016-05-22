# validations

Crate `validations` provides an interface to check the validity of arbitrary types.

* [validations](https://crates.io/crates/validations/) on crates.io
* [Documentation](https://jimmycuadra.github.io/validations/validations/) for the latest crates.io release

## Overview

The `Validate` trait provides the `validate` method, which runs arbitrary validation logic and
returns a result indicating whether or not the value is valid. A return value of `Ok(())`
indicates a valid value. A return value of `Err(Errors)` indicates an invalid value,
and includes details of why the value failed validation.

`Errors` is a container that can hold both general and field-specific validation
errors for an invalid value. An individual validation error is represented by
`Error`, which contains a human-readable error message, and an optional type of the
programmer's choice that includes additional contextual information about the error.

Types that implement `Validate` should handle validation logic for each of their fields, as
necessary. If the type of a field implements `Validate` itself, it's also possible to delegate
to the field to validate itself and assign any resulting errors back to the parent type's
errors.

Instead of implementing `Validate`, another approach is to implement validation logic inside the
constructor function of a type `T`, and return `Result<T, Errors>`, preventing an invalid value
from being created in the first place. This may not always be possible, as the value may be
created through other means. For example, the value may be deserialized from a format like JSON
from an external source. In this case, the `Validate` trait allows deserialization logic to be
decoupled from domain-level validation logic.

## Examples

Validating a value:

``` rust
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
```

Validating a value with a non-field-specific error:

``` rust
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
```

Validating a value with a field error:

``` rust
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
```

## License

[MIT](http://opensource.org/licenses/MIT)
