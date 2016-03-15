//! The driver trait which all drivers will implement.

use error::{Error, ErrorCode};
use query::{Condition, SortRule, Range, Query};
use schema::Type;
use value::{Value, ValueIter};

pub trait Driver {
  /// Connects to a driver and returns a driver instance. After calling this
  /// the driver is ready to roll!
  fn connect(uri: &str) -> Result<Self, Error> where Self: Sized;

  /// Lazily read some values from the driver.
  ///
  /// Designed against a couple of database specifications. Including the
  /// following:
  ///
  /// - [SQL `SELECT` statement][1].
  /// - [MongoDB `find` command][2].
  ///
  /// [1]: http://www.postgresql.org/docs/current/static/sql-select.html
  /// [2]: https://docs.mongodb.org/manual/reference/command/find/
  fn read(
    &self,
    type_: &Type,
    condition: Condition,
    sort: Vec<SortRule>,
    range: Range,
    query: Query
  ) -> Result<ValueIter, Error>;

  /// Read a single value from the driver. The default implementation uses the
  /// driver read method with no sort and a limit of one.
  fn read_one(
    &self,
    type_: &Type,
    condition: Condition,
    query: Query
  ) -> Result<Value, Error> {
    let mut values: Vec<_> = try!(self.read(
      type_,
      condition,
      Default::default(),
      Range::new(None, Some(1)),
      query
    )).collect();

    if values.len() > 1 {
      Err(Error::internal("Read with a limit of one returned more than one value."))
    } else if let Some(value) = values.pop() {
      Ok(value)
    } else {
      Err(Error::new(ErrorCode::NotFound, "No value was found for the condition.", None))
    }
  }
}
