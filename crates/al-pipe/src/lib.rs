mod duplex_pipe;

pub use duplex_pipe::DuplexPipe;

#[cfg(test)]
mod tests {
    use std::process::Command;
    use super::*;

    #[test]
    fn duplex_pipe() {
        // TODO: somehow test this
        let _pipe = DuplexPipe::spawn(Command::new("")).unwrap();
    }
}
