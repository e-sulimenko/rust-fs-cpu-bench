use std::{collections::BTreeMap, fs};

use apache_avro::{types::Value, Reader, Schema};
use csv::Writer as CSVWriter;

pub const INTERNAL_NULL: &str = "__NULL__";

pub fn get_avro_reader(path: &String) -> Reader<'static, fs::File> {
  let fd = fs::File::open(path).unwrap();
  let reader = Reader::new(fd).unwrap();
  return reader;
}

pub fn get_writer_schema() -> Schema {
  let raw_schema = r#"
      {
          "type": "record",
          "name": "test",
          "fields": [
              {"name": "key", "type": "string" },
              {"name": "line", "type": "string"}
          ]
      }
  "#;

  Schema::parse_str(raw_schema).expect("Parse raw_schema error")
}

pub fn get_record_from_value(value: Value) -> Vec<(String, Value)> {
  match value {
    Value::Record(record) => record,
    _ => panic!("Unexpected avro value type. Expected Value::Record."),
  }
}

pub fn parse_value(value: Value) -> Option<String> {
  match value {
    Value::String(str) => Some(str),
    Value::Union(_, v) => parse_value(*v),
    Value::Float(v) => Some(v.to_string()),
    Value::Double(v) => Some(v.to_string()),
    Value::Int(v) => Some(v.to_string()),
    Value::Long(v) => Some(v.to_string()),
    Value::Null => None,
    _ => panic!("Unexpected value type"),
  }
}

pub fn convert_row_to_string(record: &Vec<(String, Value)>) -> String {
  let mut csv_writer = CSVWriter::from_writer(vec![]);
  let csv_record = record
    .iter()
    .map(|(_, v)| match parse_value(v.clone()) {
      Some(str) => str,
      None => INTERNAL_NULL.to_string(),
    })
    .collect::<Vec<String>>();

  csv_writer.write_record(&csv_record).unwrap();

  return String::from_utf8(csv_writer.into_inner().unwrap()).unwrap();
}

pub fn get_key_str(value: &Vec<(String, Value)>, key_column: &str) -> String {
  let key_value = value.iter().find(|(name, _)| name == key_column).unwrap();

  match parse_value(key_value.1.clone()) {
    Some(str) => str,
    None => INTERNAL_NULL.to_string(),
  }
}

pub fn get_schema_from_path(path: &String) -> Schema {
  let reader = get_avro_reader(path);
  return reader.writer_schema().clone();
}

pub fn get_schema_lookup(schema: &Schema) -> BTreeMap<String, usize> {
  match schema {
    Schema::Record(schema_record) => {
      return schema_record.lookup.clone();
    }
    _ => panic!("Unexpected schema type"),
  }
}
