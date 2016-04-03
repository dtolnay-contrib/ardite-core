use bson::{Bson, Document};
use linear_map::LinearMap;
use mongodb;
use mongodb::{Client, ThreadedClient, CommandType};
use mongodb::common::{ReadPreference, ReadMode};
use mongodb::connstring;
use mongodb::db::{Database, ThreadedDatabase};
use url::Url;

use driver::Driver;
use error::Error;
use query::{Range, Sort, Condition};
use value::{Value, Iter};

pub struct MongoDB {
  database: Database
}

impl Driver for MongoDB {
  fn connect(url: &Url) -> Result<Self, Error> {
    let config = try!(connstring::parse(&url.serialize()));

    if let Some(db_name) = config.database.clone() {
      Ok(MongoDB {
        database: try!(Client::with_config(config, None, None)).db(&db_name)
      })
    } else {
      Err(Error::invalid(
        format!("Database name not provided in connection path '{}'.", url),
        "Include the database name you are connecting to as the connection URI path."
      ))
    }
  }

  fn read(
    &self,
    name: &str,
    condition: Condition,
    sort: Vec<Sort>,
    range: Range
  ) -> Result<Iter, Error> {
    let mut spec = doc! {
      "find" => name,
      "filter" => (condition_to_filter(condition)),
      "sort" => (sort_rules_to_sort(sort))
    };

    if let Some(limit) = range.limit() {
      spec.insert("limit", limit as u64);
    }
    if let Some(offset) = range.offset() {
      spec.insert("skip", offset as u64);
    }

    let cursor = try!(self.database.command_cursor(spec, CommandType::Find, ReadPreference {
      // Nearest read mode was chosen as we don’t care *too* much about stale
      // data in large usecases. Performance is more important to us. For a
      // reference on what all the read modes do, see the [documentation][1].
      //
      // Also read more about our [targeted use case][2].
      //
      // [1]: https://docs.mongodb.org/manual/reference/read-preference/#read-preference-modes
      // [2]: https://docs.mongodb.org/manual/reference/read-preference/#minimize-latency
      mode: ReadMode::Nearest,
      // Tag sets? Seems to me like they [can be ignored][1] for our use.
      //
      // [1]: https://docs.mongodb.org/manual/tutorial/configure-replica-set-tag-sets/
      tag_sets: vec![]
    }));

    Ok(Iter::new(cursor.filter_map(Result::ok).map(Value::from)))
  }
}

impl From<mongodb::Error> for Error {
  fn from(error: mongodb::Error) -> Self {
    Error::internal(format!("{}", error))
  }
}

impl From<Bson> for Value {
  /// Transformation of bson to a value. Some information is lost for
  /// non-standard types like `RegExp`, `JavaScriptCodeWithScope`, and
  /// `Binary`. The `Binary` type is completely ignored.
  #[allow(match_same_arms)]
  fn from(bson: Bson) -> Value {
    match bson {
      Bson::FloatingPoint(value) => Value::F64(value),
      Bson::String(value) => Value::String(value),
      Bson::Array(array) => Value::Array(array.into_iter().map(Value::from).collect()),
      Bson::Document(document) => Value::from(document),
      Bson::Boolean(value) => Value::Boolean(value),
      Bson::Null => Value::Null(()),
      Bson::RegExp(value, _) => Value::String(value),
      Bson::JavaScriptCode(value) => Value::String(value),
      Bson::JavaScriptCodeWithScope(value, _) => Value::String(value),
      Bson::I32(value) => Value::I64(i64::from(value)),
      Bson::I64(value) => Value::I64(value),
      Bson::TimeStamp(value) => Value::I64(i64::from(value)),
      Bson::Binary(_, _) => Value::Null(()),
      Bson::ObjectId(object_id) => Value::String(object_id.to_string()),
      Bson::UtcDatetime(time) => Value::String(time.to_rfc3339())
    }
  }
}

impl Into<Bson> for Value {
  fn into(self) -> Bson {
    match self {
      Value::Null(_) => Bson::Null,
      Value::Boolean(value) => Bson::Boolean(value),
      Value::I64(value) => Bson::I64(value),
      Value::F64(value) => Bson::FloatingPoint(value),
      Value::String(value) => Bson::String(value),
      value @ Value::Object(_) => Bson::Document(value.into()),
      Value::Array(array) => Bson::Array(array.into_iter().map(Value::into).collect())
    }
  }
}

impl From<Document> for Value {
  fn from(document: Document) -> Value {
    let mut object = LinearMap::new();
    for (key, value) in document.into_iter() {
      object.insert(key, Value::from(value));
    }
    Value::Object(object)
  }
}

impl Into<Document> for Value {
  fn into(self) -> Document {
    match self {
      Value::Object(object) => {
        let mut document = Document::new();
        for (key, value) in object.into_iter() {
          document.insert(key, value);
        }
        document
      },
      _ => Document::new()
    }
  }
}

/// Transforms an Ardite condition to a MongoDB filter as specified by the
/// MongoDB spec.
pub fn condition_to_filter(condition: Condition) -> Bson {
  match condition {
    // Because we want nested `Condition::Keys` to be represented as
    // dot-deliniated pointers (`a.b.c`) we must make sure that
    // `condition_to_filter` is only called for the highest level
    // `Condition::Keys`. For `Condition::Keys` inside `Condition::Keys` there
    // is special logic to get a flat filter document.
    Condition::Keys(keys) => {
      // This `add_keys` function is that special logic.
      fn add_keys(document: &mut Document, path: Vec<String>, keys: LinearMap<String, Condition>) {
        // For all of the keys:
        for (key, condition) in keys {
          // Create a new pointer from the parent pointer where the head is
          // the key we are looping over.
          let mut sub_path = path.clone();
          sub_path.push(key);

          if let Condition::Keys(sub_keys) = condition {
            // If the sub condition is another `Condition::Keys`, run this
            // function again instead of running `condition_to_filter`.
            add_keys(document, sub_path, sub_keys);
          } else {
            // Otherwise, insert the filter into the document at the
            // `sub_pointer`.
            document.insert(sub_path.join("."), condition_to_filter(condition));
          }
        }
      }

      let mut document = Document::new();
      add_keys(&mut document, vec![], keys);
      Bson::Document(document)
    },
    Condition::True => bson!({ "$where" => "true" }),
    Condition::False => bson!({ "$where" => "false" }),
    Condition::Not(cond) => bson!({ "$not" => (condition_to_filter(*cond)) }),
    Condition::And(conds) => bson!({
      "$and" => (Bson::Array(conds.into_iter().map(condition_to_filter).collect()))
    }),
    Condition::Or(conds) => bson!({
      "$or" => (Bson::Array(conds.into_iter().map(condition_to_filter).collect()))
    }),
    Condition::Equal(value) => {
      let bson_value: Bson = value.into();
      bson!({ "$eq" => bson_value })
    }
  }
}

/// Transform an Ardite sort to a MongoDB sort.
pub fn sort_rules_to_sort(sort_rules: Vec<Sort>) -> Bson {
  let mut document = Document::new();
  for sort_rule in sort_rules {
    document.insert(sort_rule.property().join("."), if sort_rule.is_descending() { -1 } else { 1 });
  }
  Bson::Document(document)
}

#[cfg(test)]
mod tests {
  use super::*;

  use query::{Sort, Condition};
  use value::Value;

  #[test]
  fn test_condition_to_filter() {
    let condition = Condition::Or(vec![
      Condition::True,
      Condition::False,
      Condition::And(vec![
        Condition::Not(Box::new(Condition::Equal(Value::String(str!("hello"))))),
        Condition::Equal(Value::I64(42))
      ]),
      Condition::Keys(linear_map! {
        str!("a") => Condition::False,
        str!("b") => Condition::Keys(linear_map! {
          str!("c") => Condition::Equal(Value::I64(4)),
          str!("d") => Condition::Keys(linear_map! {
            str!("e") => Condition::True
          })
        })
      })
    ]);
    let filter = bson!({
      "$or" => [
        { "$where" => "true" },
        { "$where" => "false" },
        {
          "$and" => [
            { "$not" => { "$eq" => "hello" } },
            { "$eq" => 42i64 }
          ]
        },
        {
          "a" => { "$where" => "false" },
          "b.c" => { "$eq" => 4i64 },
          "b.d.e" => { "$where" => "true" }
        }
      ]
    });
    assert_eq!(condition_to_filter(condition), filter);
  }

  #[test]
  fn test_sort_rules_to_sort() {
    let sort = vec![
      Sort::new(vec!["hello".to_owned(), "world".to_owned()], true),
      Sort::new(vec!["a".to_owned()], false)
    ];
    let sort_bson = bson!({ "hello.world" => 1, "a" => (-1) });
    assert_eq!(sort_rules_to_sort(sort), sort_bson);
  }
}
