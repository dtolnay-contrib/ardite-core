//! This module focuses on handling errors generated when using Ardite in a
//! graceful manner.

use std::io::Error as IOError;
use std::error::Error as ErrorTrait;
use std::fmt::{Display, Formatter, Result as FmtResult};

#[cfg(test)]
use regex::Regex;
use serde_json::error::Error as JSONError;
use serde_yaml::error::Error as YAMLError;

use value::{Object, Value};

/// Any error generated by Ardite or it‘s drivers should be output using this
/// type. This allows for a comprehensive display of the error when a service
/// reports it to the user.
///
/// Information included with the error includes an `ErrorCode` (which maps to
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
  code: ErrorCode,
  /// A message providing more detail beyond the error code.
  message: String,
  /// A hint to the user on what to do next to try and avoid the error
  /// happening again. This is optional.
  hint: Option<String>
}

impl Error {
  /// Easily create a new error.
  pub fn new<S>(code: ErrorCode, message: S, hint: Option<S>) -> Self where S: Into<String> {
    Error {
      code: code,
      message: message.into(),
      hint: hint.map(|string| string.into())
    }
  }

  /// Get the code for the error.
  pub fn code(&self) -> &ErrorCode {
    &self.code
  }

  /// Get the message for the error.
  pub fn message(&self) -> &str {
    &self.message
  }

  /// Get the hint—for the error (see what I did there?).
  pub fn hint(&self) -> Option<&str> {
    self.hint.as_ref().map(|s| s.as_str())
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
  /// let error = Error::new(NotFound, "Not found…", Some("Go to the light!"));
  ///
  /// let value = value!({
  ///   "error" => true,
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
    object.insert("message".to_owned(), Value::String(self.message.clone()));

    if let Some(ref hint) = self.hint {
      object.insert("hint".to_owned(), Value::String(hint.clone()));
    }

    Value::Object(object)
  }

  /// Convenience function for saying a thing failed validation using
  /// `ErrorCode::BadRequest`.
  ///
  /// # Example
  /// ```rust
  /// use ardite::error::{Error, ErrorCode};
  ///
  /// let error = Error::invalid("Failed validation.", "Try fixing your syntax!");
  ///
  /// assert_eq!(error, Error::new(ErrorCode::BadRequest, "Failed validation.", Some("Try fixing your syntax!")));
  /// ```
  pub fn invalid<S1, S2>(message: S1, hint: S2) -> Self where S1: Into<String>, S2: Into<String> {
    Error {
      code: ErrorCode::BadRequest,
      message: message.into(),
      hint: Some(hint.into())
    }
  }

  /// Convenience function for saying there was an internal error using
  /// `ErrorCode::Internal`.
  ///
  /// # Example
  /// ```rust
  /// use ardite::error::{Error, ErrorCode};
  ///
  /// let error = Error::internal("Something blew up.");
  ///
  /// assert_eq!(error, Error::new(ErrorCode::Internal, "Something blew up.", None));
  /// ```
  pub fn internal<S>(message: S) -> Self where S: Into<String> {
    Error {
      code: ErrorCode::Internal,
      message: message.into(),
      hint: None
    }
  }

  /// Convenience function for creating an unimplemented error with a plain
  /// message describing what is unimplemented using
  /// `ErrorCode::NotImplemented`.
  ///
  /// # Example
  /// ```rust
  /// use ardite::error::{Error, ErrorCode};
  ///
  /// let error = Error::unimplemented("Cache invalidation is hard.");
  ///
  /// assert_eq!(error, Error::new(ErrorCode::NotImplemented, "Cache invalidation is hard.", None));
  /// ```
  pub fn unimplemented<S>(message: S) -> Self where S: Into<String> {
    Error {
      code: ErrorCode::NotImplemented,
      message: message.into(),
      hint: None
    }
  }

  /// Special assertion for error messages. Takes a regular expression string
  /// argument which will automatically be constructed into a regualr
  /// expression. Only available in testing environments. Panics if the regular
  /// expression doesn’t match the error message string.
  #[cfg(test)]
  pub fn assert_message(&self, regex_str: &str) {
    if !Regex::new(regex_str).unwrap().is_match(&self.message) {
      panic!("Error message \"{}\" does not match regex /{}/", self.message, regex_str);
    }
  }
}

impl Display for Error {
  fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
    try!(write!(fmt, "\ncode: {}", self.code));
    try!(write!(fmt, "\nmessage: {}", self.message));

    if let Some(ref hint) = self.hint {
      try!(write!(fmt, "\nhint: {}", hint));
    }

    Ok(())
  }
}

impl ErrorTrait for Error {
  fn description(&self) -> &str {
    self.message.as_str()
  }
}

impl From<IOError> for Error {
  fn from(error: IOError) -> Self {
    Error {
      code: ErrorCode::Internal,
      message: error.description().to_owned(),
      hint: None
    }
  }
}

impl From<JSONError> for Error {
  fn from(error: JSONError) -> Self {
    match error {
      JSONError::Syntax(_, line, column) => {
        Error {
          code: ErrorCode::BadRequest,
          message: "Syntax error.".to_owned(),
          hint: Some(format!("Max sure your JSON syntax is correct around line {} column {}.", line, column))
        }
      },
      _ => {
        Error {
          code: ErrorCode::Internal,
          message: error.description().to_owned(),
          hint: None
        }
      }
    }
  }
}

impl From<YAMLError> for Error {
  fn from(error: YAMLError) -> Self {
    match error {
      YAMLError::Custom(ref message) => {
        Error {
          code: ErrorCode::BadRequest,
          message: message.to_owned(),
          hint: Some("Make sure your YAML syntax is correct.".to_owned())
        }
      },
      _ => {
        Error {
          code: ErrorCode::Internal,
          message: error.description().to_owned(),
          hint: None
        }
      }
    }
  }
}

#[cfg(feature = "error_iron")]
mod iron {
  extern crate iron;

  use super::Error;

  use self::iron::prelude::*;
  use self::iron::headers::{ContentType, ContentLength};
  use self::iron::mime::{Mime, TopLevel, SubLevel, Attr, Value};
  use self::iron::modifiers::Header;
  use self::iron::status::Status;

  impl Into<IronError> for Error {
    fn into(self) -> IronError {
      let mut res = Response::new();

      let content = self.to_value().to_json();

      res.set_mut(Header(ContentType(Mime(TopLevel::Application, SubLevel::Json, vec![(Attr::Charset, Value::Utf8)]))));
      res.set_mut(Header(ContentLength(content.len() as u64)));
      res.set_mut(Status::from_u16(self.code().to_u16()));
      res.set_mut(content);

      IronError {
        error: Box::new(self),
        response: res
      }
    }
  }
}

/// The code of an error. Designed to easily map to [HTTP status codes][1],
/// however only a subset of these codes are supported and some custom codes
/// were added.
///
/// [1]: http://www.restapitutorial.com/httpstatuscodes.html
#[derive(PartialEq, Debug)]
pub enum ErrorCode {
  /// A bad syntax was used.
  BadRequest,
  /// Permissions do not allow this to happen.
  Forbidden,
  /// Resource was not found.
  NotFound,
  /// The requested resource is not acceptable.
  NotAcceptable,
  /// Present data made the request fail.
  Conflict,
  /// There was an invalid range.
  BadRange,
  /// Something bad happened inside a driver.
  Internal,
  /// The feature has not been implemented.
  NotImplemented
}

pub use error::ErrorCode::*;

impl ErrorCode {
  pub fn to_u16(&self) -> u16 {
    match *self {
      BadRequest => 400,
      Forbidden => 403,
      NotFound => 404,
      NotAcceptable => 406,
      Conflict => 409,
      BadRange => 416,
      Internal => 500,
      NotImplemented => 501
    }
  }

  pub fn reason(&self) -> &str {
    match *self {
      BadRequest => "Bad Request",
      Forbidden => "Forbidden",
      NotFound => "Not Found",
      NotAcceptable => "Not Acceptable",
      Conflict => "Conflict",
      BadRange => "Range Not Satisfiable",
      Internal => "Internal Error",
      NotImplemented => "Not Implemented"
    }
  }
}

impl Display for ErrorCode {
  fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
    try!(write!(fmt, "{} {}", self.to_u16(), self.reason()));
    Ok(())
  }
}
