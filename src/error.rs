//! This module focuses on handling errors generated when using Ardite in a
//! graceful manner.

use std::error;
use std::io;
use std::fmt;
use std::fmt::{Display, Formatter};

use regex::Regex;
use serde_json;
use serde_yaml;

use value::{Object, Value};

/// Any error generated by Ardite or it‘s drivers should be output using this
/// type. This allows for a comprehensive display of the error when a service
/// reports it to the user.
///
/// Information included with the error includes an `Code` (which maps to
/// an HTTP status code), a message, and an optional hint telling the user how
/// to fix the error.
///
/// Typically hints should be included for what would be considered the `4xx`
/// (in HTTP language) class of error codes.
///
/// # Tips For Writing Good Hint Messages
/// - Write in the second person (“You should…”).
/// - Always recommend a solution (“You should try…”, not “You must do…”).
/// - Be as specific as possible, if you have line numbers give them. If you
///   have file paths provide them.
/// - If it is a common error which generally confuses developers, provide a
///   link to a page which better explains the error and specific steps to
///   fix it.
#[derive(PartialEq, Debug)]
pub struct Error {
  /// A specific error code which describes the error.
  code: Code,
  /// A message providing more detail beyond the error code.
  message: String,
  /// A hint to the user on what to do next to try and avoid the error
  /// happening again. This is optional.
  hint: Option<String>
}

impl Error {
  /// Easily create a new error.
  pub fn new<S>(code: Code, message: S) -> Self where S: Into<String> {
    Error {
      code: code,
      message: message.into(),
      hint: None
    }
  }

  /// Sets the error hint in a chainable fashion.
  pub fn set_hint<S>(mut self, hint: S) -> Self where S: Into<String> {
    self.hint = Some(hint.into());
    self
  }

  /// Get the code for the error.
  pub fn code(&self) -> &Code {
    &self.code
  }

  /// Get the message for the error.
  pub fn message(&self) -> &str {
    &self.message
  }

  /// Take the hint—for the error (see what I did there?).
  pub fn hint(&self) -> Option<&str> {
    self.hint.as_ref().map(|s| s.as_str())
  }

  /// Special assertion for error messages. Takes a regular expression string
  /// argument which will automatically be constructed into a regualr
  /// expression. Only available in testing environments. Panics if the regular
  /// expression doesn’t match the error message string.
  pub fn expect(&self, regex_str: &str) {
    if !Regex::new(regex_str).unwrap().is_match(&self.message) {
      panic!("Error message \"{}\" does not match regex /{}/", self.message, regex_str);
    }
  }

  /// Gets an object which represents the error.
  ///
  /// # Example
  /// ```rust
  /// #[macro_use(value)]
  /// extern crate ardite;
  ///
  /// use ardite::error::{Error, NotFound};
  ///
  /// # fn main() {
  ///
  /// let error = Error::new(NotFound, "Not found…").set_hint("Go to the light!");
  ///
  /// let value = value!({
  ///   "error" => true,
  ///   "code" => 404,
  ///   "message" => "Not found…",
  ///   "hint" => "Go to the light!"
  /// });
  ///
  /// assert_eq!(error.to_value(), value);
  ///
  /// # }
  /// ```
  pub fn to_value(&self) -> Value {
    let mut object = Object::new();

    // TODO: implement an insert method which isn‘t as strict.
    object.insert("error".to_owned(), Value::Boolean(true));
    object.insert("code".to_owned(), Value::I64(self.code().to_u16() as i64));
    object.insert("message".to_owned(), Value::String(self.message.clone()));

    if let Some(ref hint) = self.hint {
      object.insert("hint".to_owned(), Value::String(hint.clone()));
    }

    Value::Object(object)
  }

  /// Convenience function for saying a thing failed validation using
  /// `Code::BadRequest`.
  ///
  /// # Example
  /// ```rust
  /// use ardite::error::{Error, Code};
  ///
  /// let error = Error::invalid("Failed validation.", "Try fixing your syntax!");
  ///
  /// assert_eq!(error, Error::new(Code::BadRequest, "Failed validation.").set_hint("Try fixing your syntax!"));
  /// ```
  pub fn invalid<S1, S2>(message: S1, hint: S2) -> Self where S1: Into<String>, S2: Into<String> {
    Error {
      code: BadRequest,
      message: message.into(),
      hint: Some(hint.into())
    }
  }

  /// Convenience function for saying a the requested resource was not found
  /// using `Code::NotFound`.
  ///
  /// # Example
  /// ```rust
  /// use ardite::error::{Error, Code};
  ///
  /// let error = Error::not_found("Where’s Waldo?");
  ///
  /// assert_eq!(error, Error::new(Code::NotFound, "Where’s Waldo?"));
  /// ```
  pub fn not_found<S>(message: S) -> Self where S: Into<String> {
    Error {
      code: NotFound,
      message: message.into(),
      hint: None
    }
  }

  /// Convenience function for saying there was an internal error using
  /// `Code::Internal`.
  ///
  /// # Example
  /// ```rust
  /// use ardite::error::{Error, Code};
  ///
  /// let error = Error::internal("Something blew up.");
  ///
  /// assert_eq!(error, Error::new(Code::Internal, "Something blew up."));
  /// ```
  pub fn internal<S>(message: S) -> Self where S: Into<String> {
    Error {
      code: Internal,
      message: message.into(),
      hint: None
    }
  }

  /// Convenience function for creating an unimplemented error with a plain
  /// message describing what is unimplemented using
  /// `Code::NotImplemented`.
  ///
  /// # Example
  /// ```rust
  /// use ardite::error::{Error, Code};
  ///
  /// let error = Error::unimplemented("Cache invalidation is hard.");
  ///
  /// assert_eq!(error, Error::new(Code::NotImplemented, "Cache invalidation is hard."));
  /// ```
  pub fn unimplemented<S>(message: S) -> Self where S: Into<String> {
    Error {
      code: NotImplemented,
      message: message.into(),
      hint: None
    }
  }
}

impl Display for Error {
  fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
    try!(write!(fmt, "{}", self.message));

    if let Some(ref hint) = self.hint {
      try!(write!(fmt, " ({})", hint));
    }

    Ok(())
  }
}

impl error::Error for Error {
  fn description(&self) -> &str {
    self.message.as_str()
  }
}

impl From<io::Error> for Error {
  fn from(error: io::Error) -> Self {
    Error {
      code: Internal,
      message: format!("{}", error),
      hint: None
    }
  }
}

impl From<serde_json::Error> for Error {
  fn from(error: serde_json::Error) -> Self {
    match error {
      serde_json::Error::Syntax(_, line, column) => {
        Error {
          code: BadRequest,
          message: format!("{}", error),
          hint: Some(format!("Make sure your JSON syntax is correct around line {} column {}.", line, column))
        }
      },
      _ => {
        Error {
          code: Internal,
          message: format!("{}", error),
          hint: None
        }
      }
    }
  }
}

impl From<serde_yaml::Error> for Error {
  fn from(error: serde_yaml::Error) -> Self {
    match error {
      serde_yaml::Error::Custom(ref message) => {
        Error {
          code: BadRequest,
          message: message.to_owned(),
          hint: Some("Make sure your YAML syntax is correct.".to_owned())
        }
      },
      _ => {
        Error {
          code: Internal,
          message: format!("{}", error),
          hint: None
        }
      }
    }
  }
}

/// The code of an error. Designed to easily map to [HTTP status codes][1],
/// however only a subset of these codes are supported and some custom codes
/// were added.
///
/// Such a subset was decided by codes which were not specific to HTTP and had
/// universal meaning accross contexts.
///
/// **Warning:** Backwards compatibility is not guaranteed for those pattern
/// matching on this enum. The variants that exist now will *never* be removed
/// or renamed, however new variants may be added breaking any code using
/// exhaustive pattern matching.
///
/// [1]: https://en.wikipedia.org/wiki/List_of_HTTP_status_codes
#[derive(PartialEq, Debug)]
pub enum Code {
  /// 400, A bad syntax was used.
  BadRequest,
  /// 403, Permissions do not allow this to happen.
  Forbidden,
  /// 404, Resource was not found.
  NotFound,
  /// 405, Whatever method (think CRUD) used is not allowed.
  MethodNotAllowed,
  /// 406, The requested resource is not acceptable.
  NotAcceptable,
  /// 409, Present data made the request fail.
  Conflict,
  /// 416, There was an invalid range.
  BadRange,
  /// 500, Something bad happened inside a driver.
  Internal,
  /// 501, The feature has not been implemented.
  NotImplemented
}

pub use self::Code::*;

impl Code {
  /// Get the positive integer HTTP error code associated with this variant.
  /// For example the `Code::NotFound` variant would return 404.
  pub fn to_u16(&self) -> u16 {
    match *self {
      BadRequest => 400,
      Forbidden => 403,
      NotFound => 404,
      MethodNotAllowed => 405,
      NotAcceptable => 406,
      Conflict => 409,
      BadRange => 416,
      Internal => 500,
      NotImplemented => 501
    }
  }

  /// The HTTP error code “reason phrase” as defined in its specification.
  /// However, phrases which have clear references to HTTP web servers were
  /// slightly rephrased.
  pub fn reason(&self) -> &str {
    match *self {
      BadRequest => "Bad Request",
      Forbidden => "Forbidden",
      NotFound => "Not Found",
      MethodNotAllowed => "Method Not Allowed",
      NotAcceptable => "Not Acceptable",
      Conflict => "Conflict",
      BadRange => "Range Not Satisfiable",
      Internal => "Internal Error",
      NotImplemented => "Not Implemented"
    }
  }
}

impl Display for Code {
  /// Displays the error code number with its reason phrase.
  ///
  /// # Example
  /// ```rust
  /// use ardite::error::NotFound;
  ///
  /// assert_eq!(format!("{}", NotFound), "404 Not Found");
  /// ```
  fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
    try!(write!(fmt, "{} {}", self.to_u16(), self.reason()));
    Ok(())
  }
}
