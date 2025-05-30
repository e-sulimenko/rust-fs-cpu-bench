use std::fs;

use apache_avro::types::{Record, Value};
use apache_avro::{Reader, Schema, Writer};
// use base64::prelude::*;
use openssl::base64;
use openssl::rand::rand_bytes;
use openssl::symm::Cipher;

use crate::avro::{
  convert_row_to_string, get_key_str, get_record_from_value, get_schema_lookup, parse_value,
  INTERNAL_NULL,
};

pub fn generate_random_iv(cipher: Cipher) -> Vec<u8> {
  let iv_len = cipher.iv_len().unwrap_or(12); // Для AES-GCM iv_len = 12 байт

  let mut iv = vec![0u8; iv_len];
  rand_bytes(&mut iv).expect("Error to generate random bytes for iv");

  return iv;
}

pub fn encypt_avro_file(
  reader: Reader<'static, fs::File>,
  key_column: &str,
  output_path: &str,
  schema: Schema,
  mut encrypt_function: impl FnMut(String) -> Vec<u8>,
  hash_function: impl Fn(String) -> Vec<u8>,
  cleanup: bool,
) {
  let file_descriptor = fs::File::create(output_path).expect("Create encrypted file error");
  let mut result_writer = Writer::new(&schema, file_descriptor);

  for raw_value in reader {
    let value_record = get_record_from_value(raw_value.expect("Read line error"));
    // Получаем строковое представление данных ключевой колонки и всей строки
    let key_str = get_key_str(&value_record, key_column);
    let line_str = convert_row_to_string(&value_record);

    // Шифруем полученные строки, через КБ функцию
    let hashed_key = hash_function(key_str);
    let encrypted_line = encrypt_function(line_str);

    // Создаем base64 для зашифрованных строк
    let key_base64 = base64::encode_block(&hashed_key);
    let line_base64 = base64::encode_block(&encrypted_line);

    // Записываем полученную строку в авро файл
    let mut record = Record::new(&schema).expect("Empty record error");
    record.put("key", key_base64);
    record.put("line", line_base64);

    result_writer
      .append(record)
      .expect("Append encrypted line error");
    result_writer.flush().expect("Flush writer error");
  }

  if cleanup {
    fs::remove_file(output_path).expect("Remove encrypted file error");
  }
}

pub fn decrypt_avro_file(
  reader: Reader<'static, fs::File>,
  output_path: &str,
  schema: Schema,
  mut decrypt_function: impl FnMut(Vec<u8>) -> String,
  cleanup: bool,
) {
  let file_descriptor = fs::File::create(output_path).expect("Create encrypted file error");
  let mut result_writer = Writer::new(&schema, file_descriptor);
  let lookup = get_schema_lookup(&schema);

  for raw_value in reader {
    let value = raw_value.expect("Failed to unwrap value");
    let value_record = get_record_from_value(value);

    let mut record = Record::new(&schema).expect("Failed to create record");

    for (k, v) in value_record {
      if k == "key" {
        continue;
      }

      let v_str = parse_value(v).expect("Failed to parse value");
      let encoded_value = base64::decode_block(&v_str).expect("Failed to decode from base64");
      let decoded = decrypt_function(encoded_value);

      let mut csv_reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(decoded.as_bytes());

      let csv_record = csv_reader
        .records()
        .next()
        .expect("Failed to get next csv record")
        .expect("Failed to unwrap Result csv record");

      for (key, index) in &lookup {
        let value = &csv_record[*index];
        if value == INTERNAL_NULL {
          record.put(&key, Value::Union(0, Box::new(Value::Null)));
        }
        {
          record.put(
            &key,
            Value::Union(1, Box::new(Value::String(value.to_string()))),
          );
        }
      }
    }

    result_writer
      .append(record)
      .expect("Failed to append record");
    result_writer
      .flush()
      .expect("Failed to flush result writer");
  }

  if cleanup {
    fs::remove_file(output_path).expect("Remove encrypted file error");
  }
}
