use apache_avro::Schema;
use lazy_static::lazy_static;
use openssl::hash::{hash, DigestBytes, MessageDigest};
use openssl::symm::{decrypt_aead, encrypt_aead, Cipher};
use std::env;
use std::fs;
use std::ops::Div;
use std::time::{Duration, Instant};

use crate::avro::{get_avro_reader, get_schema_from_path, get_writer_schema};
use crate::crypto::{decrypt_avro_file, encypt_avro_file, generate_random_iv};

const TEST_DIR: &str = "./crypto_test";
const PRINT_WIDHT: usize = 50;

lazy_static! {
  static ref CRYPTO_TEST_RUNS_COUNT: u32 = match env::var("CRYPTO_TEST_RUNS_COUNT") {
    Ok(n) => n.parse::<u32>().unwrap_or(20),
    Err(_) => 20,
  };
}

pub struct TimeInfo {
  total_time: Duration,
  avg_time_for_run: Duration,
  max_time_for_run: Duration,
  min_time_for_run: Duration,
}

impl TimeInfo {
  fn new(time_vector: Vec<Duration>) -> TimeInfo {
    let total_time: Duration = time_vector.iter().sum();

    let avg_time_for_run: Duration = total_time.div(time_vector.len() as u32);
    let min_time_for_run: Duration = *time_vector.iter().min().unwrap();
    let max_time_for_run: Duration = *time_vector.iter().max().unwrap();

    return TimeInfo {
      total_time,
      avg_time_for_run,
      max_time_for_run,
      min_time_for_run,
    };
  }

  pub fn log(&self) {
    println!(
      "{:.<width$}{:.2?}",
      "Total runtime:",
      self.total_time,
      width = PRINT_WIDHT
    );
    println!(
      "{:.<width$}{:.2?} (min: {:.2?}, max: {:.2?})",
      "Average time in one run:",
      self.avg_time_for_run,
      self.min_time_for_run,
      self.max_time_for_run,
      width = PRINT_WIDHT
    );
  }
}

fn encrypt_aes(
  path: &String,
  output_path: &str,
  key_column: &str,
  key: &'static [u8; 32],
  aad: &'static [u8; 16],
  schema: Schema,
  cleanup: bool,
) {
  let reader = get_avro_reader(path);

  let encrypt_aead_callback = |value: String| {
    let cipher = Cipher::aes_256_gcm();

    let mut tag = vec![0; 16];
    let iv = generate_random_iv(cipher);

    let data = encrypt_aead(cipher, key, Some(&iv), aad, value.as_bytes(), &mut tag)
      .expect("Encryption failed");

    return [iv, tag, data].concat();
  };

  let hash_callback = |value: String| {
    return hash(MessageDigest::sha256(), value.as_bytes())
      .expect("Faield to hash")
      .to_vec();
  };

  encypt_avro_file(
    reader,
    key_column,
    output_path,
    schema,
    encrypt_aead_callback,
    hash_callback,
    cleanup,
  );
}

fn decrypt_aes(
  path: &String,
  output_path: &str,
  key: &'static [u8; 32],
  aad: &'static [u8; 16],
  schema: Schema,
  cleanup: bool,
) {
  let reader = get_avro_reader(path);

  let decrypt_aead_callback = |value: Vec<u8>| {
    let cipher = Cipher::aes_256_gcm();
    let iv_len = cipher.iv_len().unwrap_or(12);
    let tag_len = 16;

    let iv = &value[0..iv_len];
    let tag = &value[iv_len..(iv_len + tag_len)];
    let data = &value[(iv_len + tag_len)..];

    let decrypted =
      decrypt_aead(cipher, key, Some(iv), aad, &data, tag).expect("Decryption failed");
    String::from_utf8(decrypted).expect("Failed to convert [u8] to String")
  };

  decrypt_avro_file(reader, &output_path, schema, decrypt_aead_callback, cleanup);
}

fn encrypt_aes_test(
  path: &String,
  output_path: &str,
  key_column: &str,
  key: &'static [u8; 32],
  aad: &'static [u8; 16],
  schema: Schema,
) {
  println!("\nRunning encryption...");
  let mut times: Vec<Duration> = Vec::new();

  for run_index in 0..*CRYPTO_TEST_RUNS_COUNT {
    println!("Run #{}", run_index);
    let now = Instant::now();
    encrypt_aes(
      path,
      output_path,
      key_column,
      key,
      aad,
      schema.clone(),
      true,
    );
    times.push(now.elapsed());
  }

  TimeInfo::new(times).log();
}

fn decrypt_aes_test(
  path: &String,
  output_path: &str,
  key: &'static [u8; 32],
  aad: &'static [u8; 16],
  schema: Schema,
  cleanup: bool,
) {
  println!("\nRunning decryption...");
  let mut times: Vec<Duration> = Vec::new();

  for run_index in 0..*CRYPTO_TEST_RUNS_COUNT {
    println!("Run #{}", run_index);
    let now = Instant::now();
    decrypt_aes(path, output_path, key, aad, schema.clone(), cleanup);
    times.push(now.elapsed());
  }

  TimeInfo::new(times).log();
}

fn run_aes_benchmark() {
  println!("\nRunning AES 256 benchmarks...");

  let path = "./with_null_1.avro".to_string();
  let output_encrypted_file_path = "./encrypted.avro";
  let output_decrypted_file_path = "./decrypted.avro";
  let key_column = "phone";
  let key = b"0123456789abcdef0123456789abcdef";
  let aad = b"0123456789abcdef";

  let schema = get_writer_schema();

  encrypt_aes_test(
    &path,
    output_encrypted_file_path,
    key_column,
    key,
    aad,
    schema.clone(),
  );

  encrypt_aes(
    &path,
    output_encrypted_file_path,
    key_column,
    key,
    aad,
    schema.clone(),
    false,
  );

  let schema = get_schema_from_path(&path);

  decrypt_aes_test(
    &output_encrypted_file_path.to_string(),
    output_decrypted_file_path,
    key,
    aad,
    schema,
    true,
  );
}

pub fn run_benchmark() {
  println!(
    "\nRun Crypto benchmark using. Each test will be run {} times",
    *CRYPTO_TEST_RUNS_COUNT
  );

  let _ = fs::remove_dir_all(TEST_DIR);
  fs::create_dir(TEST_DIR).expect("Create test dir error");

  run_aes_benchmark();
}
