use lazy_static::lazy_static;
use libc::{mmap, munmap};
use std::env;
use std::fs;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::ops::AddAssign;
use std::ops::Div;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::ptr;
use std::time::Duration;
use std::time::Instant;

lazy_static! {
  static ref MAX_WRITE_FILE_SIZE: u64 = match env::var("MAX_WRITE_FILE_SIZE") {
    Ok(n) => n.parse::<u64>().unwrap_or(1_000_000_000),
    Err(_) => 1_000_000_000,
  };
  static ref MAX_READ_FILE_SIZE: u64 = match env::var("MAX_READ_FILE_SIZE") {
    Ok(n) => n.parse::<u64>().unwrap_or(20_000_000_000),
    Err(_) => 20_000_000_000,
  };
  static ref SEEK_SKIP_BYTES: u64 = match env::var("SEEK_SKIP_BYTES") {
    Ok(n) => n.parse::<u64>().unwrap_or(1024),
    Err(_) => 1024,
  };
  static ref READ_BYTES: u64 = match env::var("READ_BYTES") {
    Ok(n) => n.parse::<u64>().unwrap_or(1024),
    Err(_) => 1024,
  };
  static ref MMAP_READ_BYTES: u64 = match env::var("MMAP_READ_BYTES") {
    Ok(n) => n.parse::<u64>().unwrap_or(5_368_709_120),
    Err(_) => 5_368_709_120,
  };
  static ref FS_TEST_RUNS_COUNT: u32 = match env::var("FS_TEST_RUNS_COUNT") {
    Ok(n) => n.parse::<u32>().unwrap_or(20),
    Err(_) => 20,
  };
}

const WRITE_STRING: &str = "KDtBzffg%;K]r7D#3KcS1,UXZykBJb};v}Tp!q0B0@WU1f*hiaE1SUYExwSVhDXASHZV*.t$vGn2ph?+N=i=/KC?pT[=&Gwk+2:HS=tD!4V8rLD.aZ&TFV:nzMp/.}Qqp%fy)NP50B,,]*XrK4@$7&";
const TEST_DIR: &str = "./fs_test";
const WRITE_TEST_FILE: &str = "wirte_test_file.txt";
const READ_TEST_FILE: &str = "read_test_file.txt";
const PRINT_WIDHT: usize = 50;

fn write_file_of_size(path: &str, size: u64) {
  let mut file_descriptor = fs::File::create(&path).unwrap();

  loop {
    if file_descriptor.metadata().unwrap().size() >= size {
      break;
    }

    file_descriptor.write(WRITE_STRING.as_bytes()).unwrap();
  }
}

fn write_file(runs_count: u32) {
  println!("\nWrite file test...");

  let max_file_size = *MAX_WRITE_FILE_SIZE;

  println!(
    "During the test a file of {} bytes will be created {} times",
    max_file_size, runs_count,
  );

  let file_path = format!("{}/{}", TEST_DIR, WRITE_TEST_FILE);

  let mut total_time = Duration::ZERO;

  for _ in 0..runs_count {
    let now = Instant::now();
    write_file_of_size(&file_path, max_file_size);
    total_time.add_assign(now.elapsed());
  }

  let avg_time = total_time.div(runs_count);

  println!(
    "{:.<width$}{:.2?}",
    "Total write file runtime:",
    avg_time,
    width = PRINT_WIDHT
  );
}

fn read_file(runs_count: u32) {
  println!("\nRead file...");
  let mut total_time = Duration::ZERO;
  let mut total_read_chunks = 0;
  let read_bytes = *READ_BYTES;

  for _ in 0..runs_count {
    let mut file_descriptor = fs::File::open(format!("{}/{}", TEST_DIR, READ_TEST_FILE)).unwrap();

    let mut buf = vec![0; read_bytes.try_into().unwrap()];
    let now = Instant::now();
    loop {
      match file_descriptor.read(&mut buf) {
        Ok(n) => {
          if n == 0 {
            break;
          }
          total_read_chunks += 1;
        }
        Err(e) => panic!("{}", e),
      };
    }

    total_time.add_assign(now.elapsed());
  }
  println!(
    "{:.<width$}{:.2?}",
    format!("Total read chunks by {} bytes per run:", read_bytes),
    total_read_chunks / runs_count,
    width = PRINT_WIDHT
  );
  println!(
    "{:.<width$}{:.2?}",
    "Total read file runtime:",
    total_time.div(runs_count),
    width = PRINT_WIDHT
  );
}

fn seek_forward(runs_count: u32) {
  println!("\nSeek forward file...");
  let skip_bytes = *SEEK_SKIP_BYTES;

  let mut total_time = Duration::ZERO;
  let mut seek_jumps = 0;
  for _ in 0..runs_count {
    let mut file_descriptor = fs::File::open(format!("{}/{}", TEST_DIR, READ_TEST_FILE)).unwrap();

    let file_len = file_descriptor.metadata().unwrap().size();
    let now = Instant::now();

    loop {
      if file_descriptor
        .seek(SeekFrom::Current(skip_bytes.try_into().unwrap()))
        .unwrap()
        >= file_len
      {
        break;
      }
      seek_jumps += 1;
    }

    total_time.add_assign(now.elapsed());
  }

  println!(
    "{:.<width$}{:.2?}",
    "Seek skipping by bytes:",
    skip_bytes,
    width = PRINT_WIDHT
  );
  println!(
    "{:.<width$}{:.2?}",
    "Total seek jumps per run:",
    seek_jumps / runs_count,
    width = PRINT_WIDHT
  );
  println!(
    "{:.<width$}{:.2?}",
    "Total seek file runtime:",
    total_time.div(runs_count),
    width = PRINT_WIDHT
  );
}

fn read_file_mmap(runs_count: u32) {
  println!("\nRead file mmap...");
  let mmap_read_bytes_raw = *MMAP_READ_BYTES;

  let pages = mmap_read_bytes_raw / page_size() as u64;
  let mmap_read_bytes = pages * page_size() as u64;

  let file_path = format!("{}/{}", TEST_DIR, READ_TEST_FILE);
  let mut total_time = Duration::ZERO;
  let compare_with_sum = get_hash_sum(&file_path);
  let mut total_read_chunks = 0;
  let read_bytes = *READ_BYTES;

  for _ in 0..runs_count {
    let file_descriptor = fs::File::open(&file_path).unwrap();
    let raw_desc = file_descriptor.as_raw_fd();

    let file_size = file_descriptor.metadata().unwrap().size();

    let now = Instant::now();
    let mut hash_sum: u64 = 0;
    let mut total_read = 0;

    loop {
      if total_read >= file_size {
        break;
      }

      let len = if file_size - total_read < mmap_read_bytes as u64 {
        file_size - total_read
      } else {
        mmap_read_bytes as u64
      };

      let ptr = unsafe {
        mmap(
          ptr::null_mut(),
          len.try_into().unwrap(),
          libc::PROT_READ,
          libc::MAP_SHARED,
          raw_desc,
          total_read.try_into().unwrap(),
        )
      };

      total_read += mmap_read_bytes as u64;

      let mut data =
        unsafe { std::slice::from_raw_parts(ptr as *const u8, len.try_into().unwrap()) };

      if total_read == mmap_read_bytes {
        // Check the correctness of the first 16 bytes read
        let verify_buf = &data[0..15];
        let str = String::from_utf8(verify_buf.to_vec()).unwrap();
        assert_eq!(
          str,
          WRITE_STRING.to_string()[0..15],
          "Invalid first 16 bytes of readed buffer"
        );
      }

      let mut buf = vec![0; read_bytes.try_into().unwrap()];
      loop {
        match data.read(&mut buf) {
          Ok(n) => {
            if n == 0 {
              break;
            }
            total_read_chunks += 1;
            hash_sum += buf[0] as u64;
          }
          Err(e) => panic!("{}", e),
        }
      }

      unsafe {
        munmap(ptr, len.try_into().unwrap());
      }
    }

    total_time.add_assign(now.elapsed());
    assert_eq!(hash_sum, compare_with_sum);
  }
  println!(
    "{:.<width$}{:.2?}",
    format!("Total read chunks by {} bytes per run:", read_bytes),
    total_read_chunks / runs_count,
    width = PRINT_WIDHT
  );
  println!(
    "{:.<width$}{:.2?}",
    "Total read file runtime:",
    total_time.div(runs_count),
    width = PRINT_WIDHT
  );
}

fn page_size() -> usize {
  unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize }
}

fn get_hash_sum(path: &str) -> u64 {
  let mut file_descriptor = fs::File::open(path).unwrap();
  let mut hash_sum: u64 = 0;
  let mut buf = [0; 1024];
  loop {
    match file_descriptor.read(&mut buf) {
      Ok(n) => {
        if n == 0 {
          break;
        }
        hash_sum += buf[0] as u64;
      }
      Err(e) => panic!("{}", e),
    }
  }

  return hash_sum;
}

pub fn run_benchmark() {
  println!(
    "\nRun FS performance test. Each test will be run {} times",
    *FS_TEST_RUNS_COUNT
  );

  let _ = fs::remove_dir_all(TEST_DIR);
  let runs_count = *FS_TEST_RUNS_COUNT;

  fs::create_dir(TEST_DIR).unwrap();
  write_file(runs_count);

  println!(
    "\nCreate file for reading tests of size {} bytes",
    *MAX_READ_FILE_SIZE,
  );
  write_file_of_size(
    &format!("{}/{}", TEST_DIR, READ_TEST_FILE),
    *MAX_READ_FILE_SIZE,
  );
  read_file(runs_count);
  seek_forward(runs_count);
  read_file_mmap(runs_count);
  fs::remove_dir_all(TEST_DIR).unwrap();
}
