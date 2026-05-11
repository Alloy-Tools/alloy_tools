use std::io::{Read, Write};

const BUF_CAPACITY: usize = 4096;

fn main() {
    let mut stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut buf = [0u8; BUF_CAPACITY];
    let mut accumulator = Vec::new();
    let mut len_buf = [0u8; 2];

    loop {
        match stdin.read_exact(&mut len_buf) {
            Ok(()) => {
                let msg_len = u16::from_be_bytes(len_buf) as usize;
                if msg_len <= BUF_CAPACITY {
                    // read and echo back with buf directly
                    match stdin.read_exact(&mut buf[..msg_len]) {
                        Ok(()) => {
                            // echo buf back
                            let _ = stdout.write_all(&(msg_len as u16).to_be_bytes());
                            let _ = stdout.write_all(&buf[..msg_len]);
                            let _ = stdout.flush();
                            buf[..msg_len].fill(0);
                        }
                        Err(e) => {
                            eprintln!("[ECHO] buffer read exact error: {}", e);
                            break;
                        }
                    }
                } else {
                    //TODO: fix this path
                    // loop appending to accumulator, then echo back
                    let mut first_run = true;
                    while accumulator.len() >= 2 || first_run {
                        first_run = false;
                        match stdin.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                accumulator.extend_from_slice(&buf[..n]);

                                // if entire message, echo
                                if accumulator.len() >= msg_len {
                                    let _ = stdout.write_all(&(msg_len as u16).to_be_bytes());
                                    let _ = stdout.write_all(&accumulator[..msg_len]);
                                    let _ = stdout.flush();
                                    accumulator.drain(..msg_len);
                                }
                            }
                            Err(e) => {
                                eprintln!("[ECHO] buffer read error: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("[ECHO] message length read error: {}", e);
                break;
            }
        }
    }
}
