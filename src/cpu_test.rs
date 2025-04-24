use futures;
use lazy_static::lazy_static;
use rayon::prelude::*;
use std::env;
use std::ops::{AddAssign, Div};
use std::thread::available_parallelism;
use std::time::{Duration, Instant};
use tokio;
use tokio::runtime::Runtime;

lazy_static! {
  static ref CPU_NUM_CALCS: u64 = match env::var("CPU_NUM_CALCS") {
    Ok(n) => n.parse::<u64>().unwrap_or(10_000_00),
    Err(_) => 10_000_00,
  };
  static ref CPU_NUM_ITERS: u64 = match env::var("CPU_NUM_ITERS") {
    Ok(n) => n.parse::<u64>().unwrap_or(10_000),
    Err(_) => 10_000,
  };
  static ref CPU_TEST_RUNS_COUNT: u32 = match env::var("CPU_TEST_RUNS_COUNT") {
    Ok(n) => n.parse::<u32>().unwrap_or(20),
    Err(_) => 20,
  };
  static ref CPU_MAX_THREADS: usize = match env::var("CPU_MAX_THREADS") {
    Ok(n) => n
      .parse::<usize>()
      .unwrap_or(available_parallelism().unwrap().get()),
    Err(_) => available_parallelism().unwrap().get(),
  };
}

const PRINT_WIDHT: usize = 50;

fn factorial(num: u128) -> u128 {
  (1..=num).product()
}

fn add_one_loop(&n_loops: &u64) -> u128 {
  let mut sum = 0;
  for _in in 0..n_loops {
    sum += factorial(20);
  }

  sum
}

fn get_cpu_num() -> usize {
  *CPU_MAX_THREADS
}

fn run_native_threads(
  num_iters: u64,
  available_cores: u64,
  iter_per_core: &u64,
  total_calc: &f64,
  runs_count: u32,
) {
  println!("\nRunning in native threads...");
  let mut total_time = Duration::ZERO;
  for _ in 0..runs_count {
    let now = Instant::now();

    for _i in 0..num_iters {
      let mut results = Vec::new();
      let mut threads = Vec::new();
      for _i in 0..available_cores {
        let iter_per_core_clone = iter_per_core.clone();
        threads.push(std::thread::spawn(move || {
          add_one_loop(&iter_per_core_clone)
        }));
      }
      for thread in threads {
        results.extend(thread.join());
      }
    }

    total_time.add_assign(now.elapsed());
  }
  let avg_time = total_time.div(runs_count);
  let calc_per_sec: f64 = (total_calc) / (avg_time.as_secs() as f64);
  println!(
    "{:.<width$}{:.2?}",
    "Total native threads runtime:",
    avg_time,
    width = PRINT_WIDHT
  );
  println!(
    "{:.<width$}{:.2?}",
    "Calculations in native threads per second:",
    calc_per_sec,
    width = PRINT_WIDHT
  );
}

fn run_rayon_threads(
  num_iters: u64,
  available_cores: u64,
  iter_per_core: &u64,
  total_calc: &f64,
  runs_count: u32,
) {
  println!("\nRunning in rayon threads...");
  let mut total_time = Duration::ZERO;
  for _ in 0..runs_count {
    let now = Instant::now();

    for _i in 0..num_iters {
      (0..available_cores).into_par_iter().for_each(|_| {
        add_one_loop(iter_per_core);
      });
    }
    total_time.add_assign(now.elapsed());
  }
  let avg_time = total_time.div(runs_count);
  let calc_per_sec: f64 = (total_calc) / (avg_time.as_secs() as f64);

  println!(
    "{:.<width$}{:.2?}",
    "Total rayon threads runtime:",
    avg_time,
    width = PRINT_WIDHT
  );
  println!(
    "{:.<width$}{:.2?}",
    "Calculations in rayon threads per second:",
    calc_per_sec,
    width = PRINT_WIDHT
  );
}

fn run_tokio_threads(
  num_iters: u64,
  available_cores: u64,
  iter_per_core: &u64,
  total_calc: &f64,
  runs_count: u32,
) {
  let rt = Runtime::new().unwrap();
  rt.block_on(async {
    println!("\nRunning in tokio threads...");

    let mut total_time = Duration::ZERO;

    for _ in 0..runs_count {
      let now = Instant::now();
      for _i in 0..num_iters {
        let mut handles: Vec<tokio::task::JoinHandle<u128>> = Vec::new();
        for _i in 0..available_cores {
          let iter_per_core_clone = iter_per_core.clone();

          handles.push(tokio::spawn(
            async move { add_one_loop(&iter_per_core_clone) },
          ));
        }

        let results = futures::future::try_join_all(handles).await.unwrap();
        if results.len() == 0 {
          panic!("Result can not be 0")
        }
      }
      total_time.add_assign(now.elapsed());
    }

    let avg_time = total_time.div(runs_count);
    let calc_per_sec: f64 = (total_calc) / (avg_time.as_secs() as f64);
    println!(
      "{:.<width$}{:.2?}",
      "Total tokio threads runtime:",
      avg_time,
      width = PRINT_WIDHT
    );
    println!(
      "{:.<width$}{:.2?}",
      "Calculations in tokio threads per second:",
      calc_per_sec,
      width = PRINT_WIDHT
    );
  });
}

pub fn run_benchmark() {
  let num_calcs = *CPU_NUM_CALCS;
  let num_iters = *CPU_NUM_ITERS;
  let runs_count = *CPU_TEST_RUNS_COUNT;

  let total_calc: u64 = num_calcs * num_iters;

  println!(
    "\nRun CPU benchmark using factorial function. Each test will be run {} times",
    runs_count
  );
  println!("Number of available threads: {}", get_cpu_num());
  println!(
    "Running {} calculations over {} iterations each with a total of {} calculations.",
    &num_calcs, &num_iters, &total_calc,
  );

  let available_cores: u64 = get_cpu_num() as u64;
  let iter_per_core: u64 = num_calcs / available_cores;

  run_native_threads(
    num_iters,
    available_cores,
    &iter_per_core,
    &(total_calc as f64),
    runs_count,
  );

  run_rayon_threads(
    num_iters,
    available_cores,
    &iter_per_core,
    &(total_calc as f64),
    runs_count,
  );

  run_tokio_threads(
    num_iters,
    available_cores,
    &iter_per_core,
    &(total_calc as f64),
    runs_count,
  );
}
