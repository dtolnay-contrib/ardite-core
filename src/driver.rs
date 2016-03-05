//! This module contains the common driver code. Specific implementations for
//! different drivers exist elsewhere.

use error::{Error, ErrorCode};
use patch::Patch;
use value::{Pointer, Value};
use query::Query;

/// Gets the driver from a URL string using the protocol. For example a URL
/// of `postgres://localhost:5432/test_db` would look for a
/// `ardite-driver-postgres` crate, download the crate if it did not already
/// exist in the file system, and then return an instance initialized with the
/// `connect` static trait function.
#[allow(unused_variables)]
pub fn get_driver<D: Driver>(url: &str) -> D {
  // TODO: implement.
  unimplemented!();
}

pub trait Driver {
  /// Connects to a driver and returns a driver instance. After calling this
  /// the driver is ready to roll!
  fn connect(url: &str) -> Result<&Self, Error>;

  /// Applies multiple patches to the driver. If one patch fails, all other
  /// patches must also fail. Returns a value with all of the new patched
  /// values only.
  fn patch(&self, patch: Vec<Patch>) -> Result<Value, Error>;
  
  /// Set a value at a certain point in the driver. Returns nothing.
  fn set(&self, pointer: Pointer, value: Value) -> Result<(), Error> {
    try!(self.patch(vec![Patch::Set(pointer, value)]));
    Ok(())
  }

  /// Performs a complex query on the driver. Returns a value whose shape
  /// matches the shape of the query.
  fn query(&self, query: Query) -> Result<Value, Error>;
  
  /// Get’s a value in the driver at a specific point and returns exactly that
  /// value.
  fn get(&self, pointer: Pointer) -> Result<Value, Error> {
    match try!(self.query(Query::from(pointer.clone()))).get(pointer) {
      Some(value) => Ok(value),
      None => Err(Error {
        code: ErrorCode::Internal,
        message: String::from("Driver failed to return a value with the requested data."),
        hint: None
      })
    }
  }
}
