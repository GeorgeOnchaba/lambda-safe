use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use lambda_safe_compiler::{Compiler, CompilerConfig, SourceFormat};
use lambda_safe_solver::FiniteModelSolver;
use std::hint::black_box;
use std::sync::Arc;

fn compiler() -> Compiler {
    Compiler::new(
        Arc::new(FiniteModelSolver::default()),
        CompilerConfig::default(),
    )
}

fn generated_contract(invariants: usize) -> String {
    let mut source = String::from(
        "lambda_safe \"0.1\";\ncontract benchmark version \"1\" {\n  unit count;\n  const limit: int<count> = 2;\n  input x: int<count> authority control_plane range [0, 2];\n  assume bounded hard: x <= limit;\n",
    );
    for index in 0..invariants {
        source.push_str(&format!("  invariant inv_{index} hard: x <= limit;\n"));
    }
    source.push_str("}\n");
    source
}

fn compile_benchmark(criterion: &mut Criterion) {
    let compiler = compiler();
    let source = include_str!("../../../examples/valid/routing.lambda");
    criterion.bench_function("compile_valid_small_contract", |bencher| {
        bencher.iter(|| compiler.compile(black_box(source), SourceFormat::Lambda));
    });

    let mut group = criterion.benchmark_group("generated_invariant_scale");
    for count in [1_usize, 25, 100, 500] {
        let source = generated_contract(count);
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &source, |bencher, source| {
            bencher.iter(|| compiler.compile(black_box(source.as_str()), SourceFormat::Lambda));
        });
    }
    group.finish();
}

criterion_group!(benches, compile_benchmark);
criterion_main!(benches);
