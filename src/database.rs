//! This module contains the common driver code. Specific implementations for
//! different databases exist elsewhere.

use error::Error;
use value::*;
use query::Query;

/// Gets the database from a URL string using the protocol. For example a URL
/// of `postgres://localhost:5432/test_db` would look for a
/// `ardite-database-postgres` crate, download the crate if it did not already
/// exist in the file system, and then return an instance initialized with the
/// `connect` static trait function.
pub fn get_database<D: Database>(url: &str) -> D {
  // TODO: implement
  unimplemented!();
}

pub trait Database {
  /// Connects to a database and returns a driver instance. After calling this
  /// the driver is ready to roll!
  fn connect(url: &str) -> Self;

  /// Set a value at a certain point in the driver.
  fn set(&self, pointer: Pointer, value: Value) -> Result<(), Error>;

  /// Get a value from a certain point in the driver. An optional query may be
  /// performed for more complex data selection.
  fn get(&self, pointer: Pointer, query: Option<Query>) -> Result<Value, Error>;

  /// Gets the schema for the driver. By default no schema is returned.
  fn get_schema(&self) -> Result<Schema, Error> {
    Ok(Schema::None)
  }
}
