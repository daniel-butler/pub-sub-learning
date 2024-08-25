use criterion::{criterion_group, criterion_main, Criterion};
use std::process::{Command, Stdio};
use std::{env, process};

const INPUT_PATH: &str = "/tmp/pub-in-fifo";
const OUTPUT_PATH: &str = "/tmp/pub-out-fifo";

fn setup_pipes() {
    // Create the INPUT file if it does not exist
    if let Err(e) = nix::unistd::mkfifo(INPUT_PATH, nix::sys::stat::Mode::S_IRWXU) {
        if e != nix::errno::Errno::EEXIST {
            eprintln!("Failed to create INPUT: {}", e);
            process::exit(1);
        }
    }

    // Create the output pipe file
    if let Err(e) = nix::unistd::mkfifo(OUTPUT_PATH, nix::sys::stat::Mode::S_IRWXU) {
        if e != nix::errno::Errno::EEXIST {
            eprintln!("Failed to create INPUT: {}", e);
            process::exit(1);
        }
    }
}

fn benchmark_pub_sub(c: &mut Criterion) {
    setup_pipes();

    c.bench_function("pub_sub", |b| {
        b.iter(|| {
            let exe_filepath = env!("CARGO_BIN_EXE_pub-sub-learning");

            let mut sub_command = Command::new(exe_filepath)
                .arg("sub")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .expect("Failed to start sub");

            let mut pub_command = Command::new(exe_filepath)
                .arg("pub")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .expect("Failed to start pub");

            pub_command.kill().unwrap();
            sub_command.kill().unwrap();
        })
    });
}

criterion_group!(benches, benchmark_pub_sub);
criterion_main!(benches);
