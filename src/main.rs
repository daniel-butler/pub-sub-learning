use std::{env, process};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::OpenOptionsExt;
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};

const INPUT_PATH: &str = "/tmp/pub-in-fifo";

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    content: String,
    check_sum: Option<String>
}

impl Message {

    pub fn new(content: String) -> Self {
        match content.as_bytes().len() {
            0 => {
                Message {
                    content,
                    check_sum: None
                }
            },
            _ => {
                let mut hasher = Md5::new();
                hasher.update(content.as_bytes());
                Message {
                    content,
                    check_sum: Some(hex::encode(hasher.finalize()))
                }
            }
        }
    }

    pub fn prepare_send(&mut self) {
        // Create check sum
        let mut hasher = Md5::new();
        hasher.update(self.content.as_bytes());
        self.check_sum = Some(hex::encode(hasher.finalize()));
    }

    pub fn validate(&mut self) -> bool {
        // Check if check_sum is None. If it is, always return false
        if self.check_sum == None {
            println!("No check_sum. Always false!");
            return false;
        }

        let mut hasher = Md5::new();
        hasher.update(self.content.as_bytes());
        let temp_check_sum: String = hex::encode(hasher.finalize());

        self.check_sum == Some(temp_check_sum)
    }
}


fn main() {

    let args: Vec<String> = env::args().collect();

    if args.len() !=2 {
        eprint!("Usage: {} <pub|sub>", args[0]);
    }
    match args[1].as_str() {
        "pub" => run_pub(),
        "sub" => run_sub(),
        _ => {
            eprint!("Invalid argument. Use 'pub' or 'sub'")
        }
    }
}


fn run_pub() {
    println!("Running as pub! Type messages to sub (Ctrl+C to quit):");
    let mut output = OpenOptions::new()
        .write(true)
        .open(INPUT_PATH)
        .expect("Failed to open INPUT with write permissions!");

    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).expect("Failed to read input!");
        let input = input.trim_end();

        let mut message: Message = Message::new(input.to_string());
        message.prepare_send();

        output.write_all(serde_json::to_string(&message)
            .expect("Failed to serialize message")
            .as_bytes())
            .expect("Failed to write to INPUT");
        output.write_all(b"\n").expect("Failed to write newline to INPUT");
    }
}

fn run_sub() {
    println!("Running as sub! Waiting for messages...");

    // Create the INPUT file if it does not exist
    if let Err(e) = nix::unistd::mkfifo(INPUT_PATH, nix::sys::stat::Mode::S_IRWXU) {
        if e != nix::errno::Errno::EEXIST {
            eprint!("Failed to create FIFO: {}", e);
            process::exit(1);
        }
    }

    let input = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(INPUT_PATH)
        .expect("Failed to open INPUT for reading!");


    let mut reader = BufReader::new(input);

    loop {
        match reader.fill_buf() {
            Ok(buff) if buff.len() == 0 => {
                // No data sleep for a bit
                std::thread::sleep(std::time::Duration::from_millis(300));
            },
            Ok(_) => {
                break;
            },
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No data sleep for a bit
                std::thread::sleep(std::time::Duration::from_millis(300));
            },
            Err(e) => {
                eprint!("Error seeing if there is contents in INPUT: {}", e);
                break;
            }
        }
    }

    for line in reader.lines() {
        match line {
            Ok(raw_content) => {
                println!("Received Raw Content: {}", raw_content);
                let mut message: Message = serde_json::from_str(&raw_content)
                    .expect("Failed to parse JSON");
                if message.validate() {
                    println!("Received message: {:?} and check sum {:?}", message.content, message.check_sum.unwrap());
                } else {
                    println!("Content invalid. Check sum does not match!")
                };

            },
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No data sleep for a bit
                std::thread::sleep(std::time::Duration::from_millis(300));
            },
            Err(e) => {
                eprint!("Error reading from INPUT: {}", e);
                break;
            }
        }
    }
}