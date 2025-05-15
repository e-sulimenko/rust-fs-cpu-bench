use futures;
use lazy_static::lazy_static;
use rayon::prelude::*;
use std::env;
use std::ops::Div;
use std::thread::available_parallelism;
use std::time::{Duration, Instant};
use tokio;

lazy_static! {
  static ref CPU_NUM_CALCS: u32 = match env::var("CPU_NUM_CALCS") {
    Ok(n) => n.parse::<u32>().unwrap_or(10_000_00),
    Err(_) => 10_000_00,
  };
  static ref CPU_TEST_RUNS_COUNT: u32 = match env::var("CPU_TEST_RUNS_COUNT") {
    Ok(n) => n.parse::<u32>().unwrap_or(20),
    Err(_) => 20,
  };
  static ref CPU_MAX_THREADS: u32 = match env::var("CPU_MAX_THREADS") {
    Ok(n) => n
      .parse::<u32>()
      .unwrap_or(available_parallelism().unwrap().get().try_into().unwrap()),
    Err(_) => available_parallelism().unwrap().get().try_into().unwrap(),
  };
  static ref CPU_OPERATION_DIFFICULT_LEVEL: u32 = match env::var("CPU_OPERATION_DIFFICULT_LEVEL") {
    Ok(n) => n.parse::<u32>().unwrap_or(35),
    Err(_) => 35,
  };
}

const PRINT_WIDHT: usize = 50;

fn fibbonacci(num: u32) -> u32 {
  match num {
    0 => 0,
    1 => 1,
    _ => fibbonacci(num - 1) + fibbonacci(num - 2),
  }
}

fn get_sum_of_positive_fibbanacci_unit_fractions(num: u32) -> f64 {
  let mut result: f64 = 0f64;
  for i in 0..num {
    result += 1f64 / f64::from(fibbonacci(i));
  }
  return result;
}

fn add_one_loop(n_loops: u32) -> f64 {
  let mut sum = 0f64;
  for _in in 0..n_loops {
    sum += get_sum_of_positive_fibbanacci_unit_fractions(*CPU_OPERATION_DIFFICULT_LEVEL) as f64;
  }

  return sum;
}

fn get_cpu_num() -> u32 {
  *CPU_MAX_THREADS
}

struct TimeInfo {
  total_time: Duration,
  avg_time_for_run: Duration,
  max_time_for_run: Duration,
  min_time_for_run: Duration,
  calc_per_sec: f64,
}

impl TimeInfo {
  fn new(time_vector: Vec<Duration>, total_operation_number: u32) -> TimeInfo {
    let total_time: Duration = time_vector.iter().sum();

    let avg_time_for_run: Duration = total_time.div(time_vector.len() as u32);
    let min_time_for_run: Duration = *time_vector.iter().min().unwrap();
    let max_time_for_run: Duration = *time_vector.iter().max().unwrap();

    let calc_per_sec: f64 = f64::from(total_operation_number) / avg_time_for_run.as_secs_f64();

    return TimeInfo {
      total_time,
      avg_time_for_run,
      max_time_for_run,
      min_time_for_run,
      calc_per_sec,
    };
  }
}

fn run_native_threads(total_operation_number: u32, available_cores: u32, runs_count: u32) {
  println!("\nRunning in native threads...");

  let iter_per_core: u32 = total_operation_number.div_ceil(available_cores);
  println!("Number of iterations per thread: {}", iter_per_core);

  let mut times: Vec<Duration> = Vec::new();
  for run_index in 0..runs_count {
    println!("Run #{}", run_index);

    let now = Instant::now();
    let mut results = Vec::new();
    let mut threads = Vec::new();
    for _i in 0..available_cores {
      threads.push(std::thread::spawn(move || add_one_loop(iter_per_core)));
    }
    for thread in threads {
      results.extend(thread.join());
    }

    times.push(now.elapsed());
  }
  let time_info: TimeInfo = TimeInfo::new(times, total_operation_number);

  println!(
    "{:.<width$}{:.2?}",
    "Total runtime:",
    time_info.total_time,
    width = PRINT_WIDHT
  );
  println!(
    "{:.<width$}{:.2?} (min: {:.2?}, max: {:.2?})",
    "Average time in one run:",
    time_info.avg_time_for_run,
    time_info.min_time_for_run,
    time_info.max_time_for_run,
    width = PRINT_WIDHT
  );
  println!(
    "{:.<width$}{:.2?}",
    "Average calculations in one run per second:",
    time_info.calc_per_sec,
    width = PRINT_WIDHT
  );
}

fn run_rayon_threads(total_operation_number: u32, available_cores: u32, runs_count: u32) {
  println!("\nRunning in rayon threads...");
  let mut times: Vec<Duration> = Vec::new();
  for run_index in 0..runs_count {
    println!("Run #{}", run_index);

    let now = Instant::now();

    let scope = rayon::ThreadPoolBuilder::new()
      .num_threads(available_cores.try_into().unwrap())
      .build()
      .unwrap();

    let results = scope.install(|| -> Vec<f64> {
      return (0..total_operation_number)
        .into_par_iter()
        .map(|_| add_one_loop(1))
        .collect();
    });

    let _ = results;

    times.push(now.elapsed());
  }
  let time_info: TimeInfo = TimeInfo::new(times, total_operation_number);

  println!(
    "{:.<width$}{:.2?}",
    "Total runtime:",
    time_info.total_time,
    width = PRINT_WIDHT
  );
  println!(
    "{:.<width$}{:.2?} (min: {:.2?}, max: {:.2?})",
    "Average time in one run:",
    time_info.avg_time_for_run,
    time_info.min_time_for_run,
    time_info.max_time_for_run,
    width = PRINT_WIDHT
  );
  println!(
    "{:.<width$}{:.2?}",
    "Average calculations in one run per second:",
    time_info.calc_per_sec,
    width = PRINT_WIDHT
  );
}

fn run_tokio_threads(total_operation_number: u32, available_cores: u32, runs_count: u32) {
  println!("\nRunning in tokio threads...");

  let mut times: Vec<Duration> = Vec::new();

  for run_index in 0..runs_count {
    println!("Run #{}", run_index);

    let now = Instant::now();

    let runtime = tokio::runtime::Builder::new_multi_thread()
      .worker_threads(available_cores.try_into().unwrap())
      .thread_name("benchmark".to_string())
      .thread_stack_size(3 * 1024 * 1024)
      .build()
      .unwrap();

    let results = runtime.block_on(async move {
      let handles: Vec<tokio::task::JoinHandle<f64>> = (0..total_operation_number)
        .into_iter()
        .map(|_| tokio::spawn(async move { add_one_loop(1) }))
        .collect();

      let results = futures::future::try_join_all(handles).await.unwrap();
      if results.len() == 0 {
        panic!("Result can not be 0")
      }

      return results;
    });

    let _ = results;

    times.push(now.elapsed());
  }
  let time_info: TimeInfo = TimeInfo::new(times, total_operation_number);

  println!(
    "{:.<width$}{:.2?}",
    "Total runtime:",
    time_info.total_time,
    width = PRINT_WIDHT
  );
  println!(
    "{:.<width$}{:.2?} (min: {:.2?}, max: {:.2?})",
    "Average time in one run:",
    time_info.avg_time_for_run,
    time_info.min_time_for_run,
    time_info.max_time_for_run,
    width = PRINT_WIDHT
  );
  println!(
    "{:.<width$}{:.2?}",
    "Average calculations in one run per second:",
    time_info.calc_per_sec,
    width = PRINT_WIDHT
  );
}

pub fn run_benchmark() {
  let total_operation_number = *CPU_NUM_CALCS;
  let runs_count = *CPU_TEST_RUNS_COUNT;
  let available_cores: u32 = get_cpu_num();

  println!(
    "\nRun CPU benchmark using positive fibbanacci unit fractions series. Each test will be run {} times",
    runs_count
  );
  println!("Number of available threads: {}", get_cpu_num());

  let now = Instant::now();
  get_sum_of_positive_fibbanacci_unit_fractions(*CPU_OPERATION_DIFFICULT_LEVEL);
  let delta = now.elapsed();
  println!("\nOne operation take {:.2?}:", delta,);

  println!(
    "Running {} number of slow operations.",
    total_operation_number
  );

  run_native_threads(total_operation_number, available_cores, runs_count);

  run_rayon_threads(total_operation_number, available_cores, runs_count);

  run_tokio_threads(total_operation_number, available_cores, runs_count);
}
