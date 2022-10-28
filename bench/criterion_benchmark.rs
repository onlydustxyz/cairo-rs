use std::path::Path;

use cairo_rs::{
    cairo_run,
    hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

const BENCH_NAMES: &[&str] = &[
    "compare_arrays_200000",
    "factorial_multirun",
    "fibonacci_1000_multirun",
    "integration_builtins",
    "linear_search",
    "keccak_integration_benchmark",
    "secp_integration_benchmark",
    "blake2s_integration_benchmark",
    "dict_integration_benchmark",
    "math_integration_benchmark",
    "memory_integration_benchmark",
    "math_cmp_and_pow_integration_benchmark",
    "operations_with_data_structures_benchmarks",
    "uint256_integration_benchmark",
    "set_integration_benchmark",
];
const BENCH_PATH: &str = "cairo_programs/benchmarks/";

pub fn criterion_benchmarks(c: &mut Criterion) {
    let hint_executor = BuiltinHintProcessor::new_empty();
    for benchmark_name in build_bench_strings() {
        c.bench_function(&benchmark_name.0, |b| {
            b.iter(|| {
                cairo_run::cairo_run(
                    black_box(Path::new(&benchmark_name.1)),
                    "main",
                    false,
                    false,
                    &hint_executor,
                )
            })
        });
    }
}

fn build_bench_strings() -> Vec<(String, String)> {
    let mut full_string = Vec::<(String, String)>::new();

    for filename in BENCH_NAMES {
        let file_no_extension = String::from(*filename);
        let file_extension = String::from(".json");
        let bench_path = String::from(BENCH_PATH);
        let cairo_call = String::from("cairo_run(");
        let full_file_path = bench_path + &file_no_extension + &file_extension;
        full_string.push((cairo_call + &full_file_path.clone(), full_file_path));
    }

    full_string
}

criterion_group!(benches, criterion_benchmarks);
criterion_main!(benches);
