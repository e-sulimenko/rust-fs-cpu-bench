### Flags
If you skip all flags, all tests are run.

If you want to run test separately use following flags:
1. Runs only CPU test
```bash
--cpu
```
2. Runs only FS test
```bash
--fs
```

### ENV Variables
If you want to change the predefined configuration values: copy `.env.example` and fill it with the desired values.
```bash
cp .env.example .env
```


### Run
1. Dev mode
```bash
cargo run [-- [--cpu, --fs]]
```

2. Release mode
```bash
cargo run --release [-- [--cpu, --fs]]
```

3. Docker
```bash
docker build -t bench -f Dockerfile .
```
```bash
docker run --rm  bench
```

4. Docker occlum

Required args:
- HEAP=
- KERNEL_HEAP=
- MAX_THREADS= 
- SGX_MODE= SIM or HW

Env for tests are stored in `Occlum.json` `.env.default`.

```bash
docker build --build-arg HEAP=8GB --build-arg KERNEL_HEAP=512MB --build-arg MAX_THREADS=64 --build-arg DEBUG=false --build-arg SGX_MODE=SIM -t bench-occl -f Dockerfile.occlum .
```
```bash
docker run --rm --name bench-occl  bench-occl
```