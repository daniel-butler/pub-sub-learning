use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::{env, process};

const INPUT_PATH: &str = "/tmp/pub-in-fifo";
const OUTPUT_PATH: &str = "/tmp/pub-out-fifo";

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    content: String,
    check_sum: String,
    created_at: String,
}

impl Message {
    pub fn new(content: String) -> Self {
        let mut hasher = Md5::new();
        hasher.update(content.as_bytes());
        Message {
            content,
            check_sum: hex::encode(hasher.finalize()),
            created_at: chrono::Utc::now().to_string(),
        }
    }

    pub fn validate(&mut self) -> bool {
        let mut hasher = Md5::new();
        hasher.update(self.content.as_bytes());
        let temp_check_sum: String = hex::encode(hasher.finalize());

        self.check_sum == temp_check_sum
    }
}

fn main() {
    // Parse arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <pub|sub>", args[0]);
        println!("  pub: Publish messages");
        println!("  sub: Subscribe to messages");
    }
    match args[1].as_str() {
        "pub" => run_pub(),
        "sub" => run_sub(),
        _ => {
            println!("Invalid argument: {}. Use 'pub' or 'sub'", args[1]);
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
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read input!");
        input = input.trim_end().to_string();

        let message: Message = Message::new(input);
        output
            .write_all(
                serde_json::to_string(&message)
                    .expect("Failed to serialize message")
                    .as_bytes(),
            )
            .expect("Failed to write to INPUT");
        output
            .write_all(b"\n")
            .expect("Failed to write newline to INPUT");
    }
}

fn run_sub() {
    println!("Running as sub! Waiting for messages...");

    // Create the OUTPUT file if it does not exist
    if let Err(e) = nix::unistd::mkfifo(OUTPUT_PATH, nix::sys::stat::Mode::S_IRWXU) {
        if e != nix::errno::Errno::EEXIST {
            eprintln!("Failed to create INPUT: {}", e);
            process::exit(1);
        }
    }
    let mut output = OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(OUTPUT_PATH)
        .expect("Failed to open OUTPUT for writing!");

    // Create the INPUT file if it does not exist
    if let Err(e) = nix::unistd::mkfifo(INPUT_PATH, nix::sys::stat::Mode::S_IRWXU) {
        if e != nix::errno::Errno::EEXIST {
            eprintln!("Failed to create INPUT: {}", e);
            process::exit(1);
        }
    }

    let input = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(INPUT_PATH)
        .expect("Failed to open INPUT for reading!");

    let mut reader = BufReader::new(input);

    // Wait for the input pipe to be ready
    loop {
        match reader.fill_buf() {
            Ok([]) => {
                // No data sleep for a bit
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
            Ok(_) => {
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No data sleep for a bit
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
            Err(e) => {
                eprintln!("Error seeing if there is contents in INPUT: {}", e);
                break;
            }
        }
    }

    for line in reader.lines() {
        match line {
            Ok(raw_content) => {
                let mut message: Message =
                    serde_json::from_str(&raw_content).expect("Failed to parse JSON");

                if message.validate() {
                    println!(
                        "Received message: {:?} Check sum: {:?} Created at: {:?}",
                        message.content, message.check_sum, message.created_at
                    );
                    output
                        .write_all(
                            serde_json::to_string(&message)
                                .expect("Failed to serialize message")
                                .as_bytes(),
                        )
                        .expect("Failed to write to OUTPUT");
                } else {
                    println!("Content invalid. Check sum does not match!")
                };
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No data sleep for a bit
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
            Err(e) => {
                eprintln!("Error reading from INPUT: {}", e);
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_new() {
        let message = Message::new("Hello World".to_string());
        assert_eq!(message.content, "Hello World");
        assert_eq!(message.check_sum, "b10a8db164e0754105b7a99be72e3fe5");
    }

    #[test]
    fn test_message_validate() {
        let mut message = Message::new("Hello World".to_string());
        assert_eq!(message.validate(), true);

        message.check_sum = "invalid".to_string();
        assert_eq!(message.validate(), false);
    }

    #[test]
    fn test_serialize_and_deserialize() {
        let message = Message::new("Hello World".to_string());
        let json = serde_json::to_string(&message).unwrap();
        let message2: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(message.content, message2.content);
        assert_eq!(message.check_sum, message2.check_sum);
        assert_eq!(message.created_at, message2.created_at);
    }
}
