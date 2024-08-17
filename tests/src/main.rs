use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::{env, process};

const OUTPUT_PATH: &str = "/tmp/pub-out-fifo";

#[test]
fn test_pub_sub() {
    // Create the output pipe file
    if let Err(e) = nix::unistd::mkfifo(OUTPUT_PATH, nix::sys::stat::Mode::S_IRWXU) {
        if e != nix::errno::Errno::EEXIST {
            eprintln!("Failed to create INPUT: {}", e);
            process::exit(1);
        }
    }

    // Start the pub and sub processes
    let exe_filepath = env!("CARGO_BIN_EXE_pub-sub-learning");
    let mut sub_command = Command::new(exe_filepath)
        .arg("sub")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start pub");
    let mut pub_command = Command::new(exe_filepath)
        .arg("pub")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start sub");

    // Wait for simulated user input
    pub_command
        .stdin
        .as_mut()
        .unwrap()
        .write_all("Hello World\n".as_bytes())
        .unwrap();

    // get the message from the output file
    let mut reader = BufReader::new(OpenOptions::new().read(true).open(OUTPUT_PATH).unwrap());

    let mut output_string = String::new();
    reader
        .fill_buf()
        .expect("Failed to read line")
        .read_to_string(&mut output_string)
        .expect("Failed to read line");
    println!("Received output string: {}", output_string);
    assert!(output_string.contains("Hello World"));
    assert!(output_string.contains("b10a8db164e0754105b7a99be72e3fe5"));

    // Clean up
    pub_command.kill().unwrap();
    sub_command.kill().unwrap();
}
