mod cpu_test;
mod fs_test;
use dotenv::dotenv;

const CPU_FLAG: &str = "--cpu";
const FS_FLAG: &str = "--fs";

fn main() {
  dotenv().ok();

  let args = std::env::args().collect::<Vec<String>>();
  let is_cpu_test = args.contains(&CPU_FLAG.to_string());
  let is_fs_test = args.contains(&FS_FLAG.to_string());

  let is_all = !is_cpu_test && !is_fs_test;

  if is_cpu_test || is_all {
    cpu_test::run_benchmark();
  }
  if is_fs_test || is_all {
    fs_test::run_benchmark();
  }
}
