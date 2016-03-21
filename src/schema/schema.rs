//! Format for defining the shape of data in an Ardite Schema Definition.

use std::fmt::Debug;
use std::ops::Deref;
use linear_map::LinearMap;
use regex::Regex;

use error::Error;
use query::Query;
use value::{Key, Pointer, Value};

lazy_static! {
  static ref INTEGER_RE: Regex = Regex::new(r"^\d+$").unwrap();
}

// TODO: use newtype pattern and expose all of the `Schema` functions through this?
pub type BoxedSchema = Box<Schema + 'static>;

/// A schema detailing what the data received from the driver (or inserted
/// into the driver) should be. To describe this data we use a subset of
/// [JSON Schema][1]. The schema is a subset of JSON Schema for three reasons:
///
/// 1. Searchability throughout the schema. It must be possible to do
///    `schema.get("/hello/world")` which finds an object schema, for example
///    with the `hello` property and then another nested `world` property.
///    Nested schemas must be retrievable and this goal is not possible with
///    JSON Schema constructs like `oneOf`, `allOf`, `noneOf`, or `not` make it
///    difficult (if not impossible) to find a single schema for a pointer.
///
/// 2. Schema extension. In some areas, adding new properties to the schema
///    which don’t have strict validation purposes is useful. For example
///    `$type`, `$gen`, or `key`.
///
/// 3. Easy interoperability with a Rust enum. In Rust-land the best way to
///    represent a schema like this is with an enum. The official JSON
///    meta-schema and specification do not provide a format to easily
///    transform to a Rust enum format, therefore a custom definition is
///    required.
///
/// [1]: http://json-schema.org
pub trait Schema: Debug {
  /// Used to get a nested schema at a certain point.
  fn get(&self, mut pointer: Pointer) -> Option<&Schema>;

  /// Validates a query that a user would like to make on the database by
  /// comparing it to the schema. Mostly checks that all properties described
  /// in the query are accessible according to the schema.
  fn validate_query(&self, query: &Query) -> Result<(), Error>;
}

// TODO: find way to doc usable methods from `SchemaNone`, `SchemaNull`, etc.
impl Schema {
  /// Create a schema which does not run any validations.
  pub fn none() -> SchemaNone {
    SchemaNone::new()
  }

  /// Create a schema which ensures the value is null and only null.
  pub fn null() -> SchemaNull {
    SchemaNull::new()
  }

  /// Create a schema which validates a boolean primitive type.
  pub fn boolean() -> SchemaBoolean {
    SchemaBoolean::new()
  }

  /// Create a schema which validates a number.
  pub fn number() -> SchemaNumber {
    SchemaNumber::new()
  }

  /// Create a schema which validates a string.
  pub fn string() -> SchemaString {
    SchemaString::new()
  }

  /// Create a schema which validates an array. It takes another schema to
  /// validate all of the child properties.
  pub fn array() -> SchemaArray {
    SchemaArray::new()
  }

  /// Create a schema which validates an object.
  pub fn object() -> SchemaObject {
    SchemaObject::new()
  }

  /// Creates a schema which validates enumerated values.
  pub fn enum_<V>(values: Vec<V>) -> SchemaEnum where V: Into<Value> {
    SchemaEnum::new(values.into_iter().map(Into::into).collect())
  }
}

// We need equality checks in our tests, however, comparing equality on a trait
// object is hard. Therefore, since this isn’t a production grade check, we
// compare the strings generated by `Debug`.
#[cfg(test)]
impl<'a, 'b> PartialEq<Schema + 'b> for Schema + 'a {
  fn eq(&self, other: &(Schema + 'b)) -> bool {
    format!("{:?}", self) == format!("{:?}", other)
  }
}

pub trait SchemaPrimitive: Debug {}

impl<'a, T> Schema for T where T: SchemaPrimitive + 'a {
  fn get(&self, pointer: Pointer) -> Option<&Schema> {
    if pointer.is_empty() {
      Some(self)
    } else {
      None
    }
  }

  fn validate_query(&self, query: &Query) -> Result<(), Error> {
    match *query {
      Query::All => Ok(()),
      Query::Keys(_) => Err(Error::invalid(
        "Cannot deeply query a primitive value.",
        "Try not querying specific properties of a primitive like `null` or `boolean`."
      ))
    }
  }
}

/// There is no schema. No validations should occur. Does not represent the
/// abscense of any value, only represents that a schema does not define the
/// data structure at this point.
#[derive(Debug)]
pub struct SchemaNone;

impl SchemaNone {
  pub fn new() -> Self {
    SchemaNone
  }
}

impl Schema for SchemaNone {
  fn get(&self, pointer: Pointer) -> Option<&Schema> {
    if pointer.is_empty() {
      Some(self)
    } else {
      None
    }
  }

  fn validate_query(&self, _: &Query) -> Result<(), Error> {
    Ok(())
  }
}

/// Represents the absence of any value.
#[derive(Debug)]
pub struct SchemaNull;

impl SchemaNull {
  pub fn new() -> Self {
    SchemaNull
  }
}

impl SchemaPrimitive for SchemaNull {}

/// Represents a binary true/false value.
#[derive(Debug)]
pub struct SchemaBoolean;

impl SchemaBoolean {
  pub fn new() -> Self {
    SchemaBoolean
  }
}

impl SchemaPrimitive for SchemaBoolean {}

/// Represents a numeric type.
#[derive(Debug)]
pub struct SchemaNumber {
  /// Forces the number to be a multiple of another. This helps in specifying
  /// integers if this value is `Some(1)` for example.
  multiple_of: Option<f32>,
  /// The minimum value the number can be.
  minimum: Option<f64>,
  /// Whether or not the minimum value should be included when validating.
  /// Default is `false`.
  exclusive_minimum: bool,
  /// The maximum value the number can be.
  maximum: Option<f64>,
  /// Whether or not the maximum value should be included when validating.
  /// Default is `false`.
  exclusive_maximum: bool
}

impl SchemaNumber {
  pub fn new() -> Self {
    SchemaNumber {
      multiple_of: None,
      minimum: None,
      exclusive_minimum: false,
      maximum: None,
      exclusive_maximum: false
    }
  }

  pub fn set_multiple_of(&mut self, multiple_of: f32) { self.multiple_of = Some(multiple_of); }
  pub fn set_minimum(&mut self, minimum: f64) { self.minimum = Some(minimum); }
  pub fn enable_exclusive_minimum(&mut self) { self.exclusive_minimum = true; }
  pub fn set_maximum(&mut self, maximum: f64) { self.maximum = Some(maximum); }
  pub fn enable_exclusive_maximum(&mut self) { self.exclusive_maximum = true; }
  pub fn multiple_of(&self) -> Option<f32> { self.multiple_of }
  pub fn minimum(&self) -> Option<f64> { self.minimum }
  pub fn exclusive_minimum(&self) -> bool { self.exclusive_minimum }
  pub fn maximum(&self) -> Option<f64> { self.maximum }
  pub fn exclusive_maximum(&self) -> bool { self.exclusive_maximum }
}

impl SchemaPrimitive for SchemaNumber {}

/// Represents a string type.
#[derive(Debug)]
pub struct SchemaString {
  /// The mimimum length of characters in the string.
  min_length: Option<u64>,
  /// The maximum length of characters in the string.
  max_length: Option<u64>,
  /// A regular expression pattern to validate the string against.
  pattern: Option<Regex>
}

impl SchemaString {
  pub fn new() -> Self {
    SchemaString {
      min_length: None,
      max_length: None,
      pattern: None
    }
  }

  pub fn set_min_length(&mut self, min_length: u64) { self.min_length = Some(min_length); }
  pub fn set_max_length(&mut self, max_length: u64) { self.max_length = Some(max_length); }
  pub fn set_pattern(&mut self, pattern: Regex) { self.pattern = Some(pattern); }
  pub fn min_length(&self) -> Option<u64> { self.min_length }
  pub fn max_length(&self) -> Option<u64> { self.max_length }
  pub fn pattern(&self) -> Option<&Regex> { self.pattern.as_ref() }
}

impl SchemaPrimitive for SchemaString {}

/// Represents a set of any type.
#[derive(Debug)]
pub struct SchemaArray {
  /// A schema which all items in the array must match.
  // We use box because the array must take ownership of its child schema.
  items: Option<BoxedSchema>
}

impl SchemaArray {
  pub fn new() -> Self {
    SchemaArray {
      items: None
    }
  }

  pub fn set_items<S>(&mut self, schema: S) where S: Schema + 'static {
    self.items = Some(Box::new(schema));
  }

  pub fn set_boxed_items(&mut self, schema: BoxedSchema) {
    self.items = Some(schema);
  }

  pub fn items(&self) -> Option<&Schema> {
    self.items.as_ref().map(|schema| schema.deref())
  }
}

impl Schema for SchemaArray {
  fn get(&self, mut pointer: Pointer) -> Option<&Schema> {
    if pointer.is_empty() {
      Some(self)
    } else {
      if INTEGER_RE.is_match(&pointer.remove(0)) {
        if let Some(ref items) = self.items {
          items.get(pointer)
        } else {
          None
        }
      } else {
        None
      }
    }
  }

  fn validate_query(&self, query: &Query) -> Result<(), Error> {
    match *query {
      Query::All => Ok(()),
      Query::Keys(ref query_properties) => {
        let err_key = query_properties.keys().map(|key| {
          if INTEGER_RE.is_match(key) {
            if let Some(ref items) = self.items {
              items.validate_query(&query_properties.get(key).unwrap())
            } else {
              Ok(())
            }
          } else {
            Err(Error::invalid(format!("Cannot query non-integer \"{}\" array property.", key), "Only query integer array keys like 1, 2, and 3."))
          }
        }).find(|r| r.is_err());
        match err_key {
          None => Ok(()),
          Some(error) => error
        }
      }
    }
  }
}

/// Represents a set of key/value pairs.
#[derive(Debug)]
pub struct SchemaObject {
  /// Schemas associated to the object properties.
  // We use box because the object must take ownership of its child schema.
  properties: LinearMap<Key, BoxedSchema>,
  /// Properties that are required to be in the object.
  required: Vec<Key>,
  /// Whether or not there may be extra properties outside of the ones
  /// defined by the properties map.
  additional_properties: bool
}

impl SchemaObject {
  pub fn new() -> Self {
    SchemaObject {
      properties: LinearMap::new(),
      required: Vec::new(),
      additional_properties: false
    }
  }

  pub fn add_property<K, S>(&mut self, key: K, schema: S) where K: Into<Key>, S: Schema + 'static {
    self.properties.insert(key.into(), Box::new(schema));
  }

  pub fn add_boxed_property<K>(&mut self, key: K, schema: BoxedSchema) where K: Into<Key> {
    self.properties.insert(key.into(), schema);
  }

  pub fn set_required<K>(&mut self, required: Vec<K>) where K: Into<Key> {
    self.required = required.into_iter().map(Into::into).collect();
  }

  pub fn enable_additional_properties(&mut self) {
    self.additional_properties = true;
  }

  pub fn properties(&self) -> LinearMap<Key, &Schema> {
    let mut properties = LinearMap::new();
    for (key, value) in self.properties.iter() {
      properties.insert(key.to_owned(), value.deref());
    }
    properties
  }

  pub fn required(&self) -> &Vec<Key> {
    &self.required
  }

  pub fn additional_properties(&self) -> bool {
    self.additional_properties
  }
}

impl Schema for SchemaObject {
  fn get(&self, mut pointer: Pointer) -> Option<&Schema> {
    if pointer.is_empty() {
      Some(self)
    } else {
      if let Some(schema) = self.properties.get(&pointer.remove(0)) {
        schema.get(pointer)
      } else {
        None
      }
    }
  }

  fn validate_query(&self, query: &Query) -> Result<(), Error> {
    match *query {
      Query::All => Ok(()),
      Query::Keys(ref query_properties) => {
        let err_key = query_properties.keys().map(|key| {
          if let Some(property_schema) = self.properties.get(key) {
            property_schema.validate_query(&query_properties.get(key).unwrap())
          } else if self.additional_properties {
            Ok(())
          } else {
            Err(Error::invalid(format!("Cannot query object property \"{}\".", key), "Query an object property that is defined in the schema."))
          }
        }).find(|r| r.is_err());
        match err_key {
          None => Ok(()),
          Some(error) => error
        }
      }
    }
  }
}

/// Represents a value which *must* be one of the defined values. An enum is
/// considered a primitive type as if it is a single value is a higher order
/// type, no variation is allowed.
#[derive(Debug)]
pub struct SchemaEnum {
  /// The available values.
  values: Vec<Value>
}

impl SchemaEnum {
  pub fn new(values: Vec<Value>) -> Self {
    SchemaEnum {
      values: values
    }
  }

  pub fn values(&self) -> &Vec<Value> {
    &self.values
  }
}

impl SchemaPrimitive for SchemaEnum {}

#[cfg(test)]
mod tests {
  use schema::Schema;
  use query::Query;

  #[test]
  fn test_schema_equality() {
    let mut object_2_raw = Schema::object();
    object_2_raw.set_required(vec!["hello", "a"]);
    object_2_raw.enable_additional_properties();
    object_2_raw.add_property("goodbye", Schema::boolean());
    let mut object_3_raw = Schema::object();
    object_3_raw.add_property("hello", Schema::number());
    object_3_raw.add_property("goodbye", {
      let mut object = Schema::object();
      object.add_property("a", Schema::number());
      object
    });
    let mut array_1_raw = Schema::array();
    array_1_raw.set_items({
      let mut object = Schema::object();
      object.add_property("num", Schema::number());
      object
    });
    let mut array_2_raw = Schema::array();
    array_2_raw.set_items(Schema::number());

    let number: Box<Schema> = Box::new(Schema::number());
    let object_1: Box<Schema> = Box::new(Schema::object());
    let object_2: Box<Schema> = Box::new(object_2_raw);
    let object_3: Box<Schema> = Box::new(object_3_raw);
    let array_1: Box<Schema> = Box::new(array_1_raw);
    let array_2: Box<Schema> = Box::new(array_2_raw);

    let schemas: Vec<Box<Schema>> = vec![number, object_1, object_2, object_3, array_1, array_2];

    for schema_1 in &schemas {
      assert!(schema_1.eq(schema_1));
      let mut equals = 0;
      for schema_2 in &schemas {
        if schema_1.eq(schema_2) {
          if equals > 0 {
            panic!("Schema should only equal itself!");
          } else {
            equals += 1;
          }
        }
      }
      assert_eq!(equals, 1);
    }
  }

  #[test]
  fn test_get_primitive() {
    assert!(Schema::none().get(point![]).unwrap().eq(&Schema::none()));
    assert!(Schema::none().get(point!["hello"]).is_none());
    assert!(Schema::boolean().get(point![]).unwrap().eq(&Schema::boolean()));
    assert!(Schema::boolean().get(point!["hello"]).is_none());
    assert!(Schema::number().get(point!["hello"]).is_none());
    assert!(Schema::string().get(point!["hello"]).is_none());
  }

  #[test]
  fn test_get_array() {
    let array_none = Schema::array();
    let mut array_bool = Schema::array();
    array_bool.set_items(Schema::boolean());
    assert!(array_none.get(point!["1"]).is_none());
    assert!(array_none.get(point!["asd"]).is_none());
    assert!(array_bool.get(point!["1"]).unwrap().eq(&Schema::boolean()));
    assert!(array_bool.get(point!["9999999"]).unwrap().eq(&Schema::boolean()));
    assert!(array_bool.get(point!["asd"]).is_none());
  }

  #[test]
  fn test_get_object() {
    let mut object = Schema::object();
    object.add_property("hello", Schema::boolean());
    object.add_property("world", Schema::boolean());
    object.add_property("5", Schema::boolean());
    object.add_property("goodbye", {
      let mut goodbye = Schema::object();
      goodbye.add_property("hello", Schema::boolean());
      goodbye.add_property("world", Schema::boolean());
      goodbye
    });
    assert!(object.get(point!["yo"]).is_none());
    assert!(object.get(point!["hello"]).unwrap().eq(&Schema::boolean()));
    assert!(object.get(point!["goodbye", "world"]).unwrap().eq(&Schema::boolean()));
    assert!(object.get(point!["goodbye", "yo"]).is_none());
  }

  #[test]
  fn test_query_none() {
    assert!(Schema::none().validate_query(&Query::All).is_ok());
    assert!(Schema::none().validate_query(&Query::Keys(linear_map! {
      str!("s@#f&/Ij)82h(;pa0]") => Query::All,
      str!("123") => Query::All,
      str!("hello") => Query::All,
      str!("nested") => Query::Keys(linear_map! {
        str!("yo") => Query::All
      })
    })).is_ok());
  }

  #[test]
  fn test_query_primitive() {
    assert!(Schema::null().validate_query(&Query::All).is_ok());
    let obj_query = Query::Keys(linear_map! {});
    Schema::null().validate_query(&obj_query).unwrap_err().assert_message("deeply query");
    Schema::boolean().validate_query(&obj_query).unwrap_err().assert_message("deeply query");
    Schema::number().validate_query(&obj_query).unwrap_err().assert_message("deeply query");
    Schema::string().validate_query(&obj_query).unwrap_err().assert_message("deeply query");
    Schema::enum_(vec![true, false]).validate_query(&obj_query).unwrap_err().assert_message("deeply query");
  }

  #[test]
  fn test_query_array() {
    let array_none = Schema::array();
    let mut array_bool = Schema::array();
    array_bool.set_items(Schema::boolean());
    assert!(array_none.validate_query(&Query::All).is_ok());
    assert!(array_none.validate_query(&Query::Keys(linear_map! {
      str!("1") => Query::All
    })).is_ok());
    assert!(array_none.validate_query(&Query::Keys(linear_map! {
      str!("1") => Query::Keys(linear_map! {})
    })).is_ok());
    assert!(array_bool.validate_query(&Query::Keys(linear_map! {
      str!("1") => Query::All,
      str!("2") => Query::All,
      str!("3") => Query::All,
      str!("50") => Query::All,
      str!("9999999999999") => Query::All
    })).is_ok());
    array_none.validate_query(&Query::Keys(linear_map! {
      str!("hello") => Query::All
    })).unwrap_err().assert_message("non-integer \"hello\"");
    array_bool.validate_query(&Query::Keys(linear_map! {
      str!("1") => Query::Keys(linear_map! {})
    })).unwrap_err().assert_message("deeply query");
  }

  #[test]
  fn test_query_object() {
    let mut object = Schema::object();
    object.add_property("hello", Schema::boolean());
    object.add_property("world", Schema::boolean());
    object.add_property("5", Schema::boolean());
    object.add_property("goodbye", {
      let mut goodbye = Schema::object();
      goodbye.add_property("hello", Schema::boolean());
      goodbye.add_property("world", Schema::boolean());
      goodbye
    });
    let mut object_additional = Schema::object();
    object_additional.enable_additional_properties();
    object_additional.add_property("hello", Schema::boolean());
    object_additional.add_property("world", Schema::boolean());
    assert!(object.validate_query(&Query::Keys(linear_map! {
      str!("world") => Query::All,
      str!("5") => Query::All,
      str!("goodbye") => Query::All
    })).is_ok());
    object.validate_query(&Query::Keys(linear_map! {
      str!("hello") => Query::All,
      str!("moon") => Query::All
    })).unwrap_err().assert_message("Cannot query object property \"moon\".");
    object.validate_query(&Query::Keys(linear_map! {
      str!("hello") => Query::Keys(linear_map! {})
    })).unwrap_err().assert_message("deeply query");
    assert!(object.validate_query(&Query::Keys(linear_map! {
      str!("goodbye") => Query::Keys(linear_map! {
        str!("hello") => Query::All
      })
    })).is_ok());
    object.validate_query(&Query::Keys(linear_map! {
      str!("goodbye") => Query::Keys(linear_map! {
        str!("hello") => Query::Keys(linear_map! {})
      })
    })).unwrap_err().assert_message("deeply query");
    assert!(object_additional.validate_query(&Query::Keys(linear_map! {
      str!("world") => Query::All,
      str!("5") => Query::All,
      str!("goodbye") => Query::All,
      str!("moon") => Query::All
    })).is_ok());
  }
}
