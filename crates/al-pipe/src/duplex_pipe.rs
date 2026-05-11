use al_core::{BinarySerde, Command, Queue, SerdeFormat, Transport, TransportError};
use std::{
    io::{Read, Write},
    process::{Child, Command as Process, Stdio},
    sync::Arc,
    time::Duration,
};

/// Status of individual reader/writer threads
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DuplexPipeStatus {
    pub write_thread_finished: bool,
    pub read_thread_finished: bool,
    pub pipe_alive: bool,
}

#[derive(Debug)]
pub struct DuplexPipe {
    incoming: Arc<Queue<Command>>,
    outgoing: Arc<Queue<Command>>,
    threads: (
        tokio::task::JoinHandle<Result<(), String>>,
        tokio::task::JoinHandle<Result<(), String>>,
    ),
    #[allow(unused)]
    child: Child,
}

impl DuplexPipe {
    pub fn spawn(command: Process) -> Result<Self, std::io::Error> {
        Self::spawn_with_timeout(command, None)
    }

    pub fn spawn_with_timeout(
        mut command: Process,
        timeout: Option<Duration>,
    ) -> Result<Self, std::io::Error> {
        // Set up stdio pipes
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        let mut child = command.spawn()?;
        let pipe_child_in = child
            .stdin
            .take()
            .expect("Child should always have stdin on spawn");
        let pipe_child_out = child
            .stdout
            .take()
            .expect("Child should always have stdout on spawn");

        // Set up queues
        let queue_child_in = Arc::new(Queue::new());
        let queue_child_out = Arc::new(Queue::new());

        // Spawn writer thread
        let write_queue = queue_child_in.clone();
        let write_thread: tokio::task::JoinHandle<Result<(), String>> = tokio::spawn(async move {
            let mut stdin = pipe_child_in;
            let serializer = BinarySerde;

            loop {
                match write_queue.recv().await {
                    Ok(cmd) => {
                        let buf = serializer
                            .serialize_command(&cmd)
                            .map_err(|e| e.to_string())?;

                        // Wrap write operations with timeout if specified
                        let write_op = async {
                            // Write buffer length
                            if let Err(e) = tokio::task::block_in_place(|| {
                                stdin.write_all(&(buf.len() as u16).to_be_bytes())
                            }) {
                                let err = format!(
                                    "[DUPLEX PIPE | WRITE THREAD] Length write error: {}",
                                    e
                                );
                                eprintln!("{}", err);
                                return Err(err);
                            }
                            // Write buffer
                            if let Err(e) = tokio::task::block_in_place(|| stdin.write_all(&buf)) {
                                let err = format!(
                                    "[DUPLEX PIPE | WRITE THREAD] Payload write error: {}",
                                    e
                                );
                                eprintln!("{}", err);
                                return Err(err);
                            }
                            // Flush
                            if let Err(e) = tokio::task::block_in_place(|| stdin.flush()) {
                                let err =
                                    format!("[DUPLEX PIPE | [WRITE THREAD] Flush error: {}", e);
                                eprintln!("{}", err);
                                return Err(err);
                            }
                            Ok(())
                        };

                        match timeout {
                            Some(duration) => {
                                match tokio::time::timeout(duration, write_op).await {
                                    Ok(result) => result?,
                                    Err(_) => {
                                        return Err(
                                            "Write timeout: subprocess did not accept data in time"
                                                .to_string(),
                                        )
                                    }
                                }
                            }
                            None => write_op.await?,
                        }
                    }
                    Err(e) => {
                        let err = format!(
                            "[DUPLEX PIPE | WRITE THREAD] Error receiving from queue: {:?}",
                            e
                        );
                        eprintln!("{}", err);
                        Err(err)?
                    }
                }
            }
        });

        // spawn reader thread
        let read_queue = queue_child_out.clone();
        let read_thread: tokio::task::JoinHandle<Result<(), String>> = tokio::spawn(async move {
            let mut stdout = pipe_child_out;
            let mut buf = [0u8; 1024];
            let mut len_bytes = [0u8; 2];
            let mut len: usize;
            let serializer = BinarySerde;

            loop {
                // Read two bytes as message length
                match tokio::task::block_in_place(|| stdout.read_exact(&mut len_bytes)) {
                    Ok(()) => {
                        len = u16::from_be_bytes(len_bytes) as usize;
                        let read_op = async {
                            // Read `message length` bytes as message
                            tokio::task::block_in_place(|| stdout.read_exact(&mut buf[..len]))
                                .map_err(|e| {
                                    let err = format!(
                                        "[DUPLEX PIPE | READ THREAD] Payload read error: {}",
                                        e
                                    );
                                    eprintln!("{}", err);
                                    err
                                })?;
                            // Deserialize message into a command
                            let cmd = serializer.deserialize_command(&buf[..len]).map_err(|e| {
                                let err = format!(
                                    "[DUPLEX PIPE | READ THREAD] Deserialization error: {}",
                                    e
                                );
                                eprintln!("{}", err);
                                err
                            })?;
                            // Forward command to incoming queue
                            if let Err(e) = read_queue.send(cmd).await {
                                let err = format!(
                                    "[DUPLEX PIPE | READ THREAD] Send to queue error: {:?}",
                                    e
                                );
                                eprintln!("{}", err);
                                return Err(err);
                            }
                            Ok(())
                        };

                        match timeout {
                            Some(duration) => match tokio::time::timeout(duration, read_op).await {
                                Ok(result) => result?,
                                Err(_) => {
                                    return Err(
                                        "Read timeout: subprocess did not send data in time"
                                            .to_string(),
                                    )
                                }
                            },
                            None => read_op.await?,
                        }
                    }
                    Err(e) => {
                        match e.kind() {
                            std::io::ErrorKind::UnexpectedEof => {
                                eprintln!("[DUPLEX PIPE | READ THREAD] EOF reached: {}", e)
                            }
                            _ => {} //TODO
                        };
                        break;
                    }
                }
            }
            Ok(())
        });

        Ok(Self {
            incoming: queue_child_out,
            outgoing: queue_child_in,
            threads: (write_thread, read_thread),
            child,
        })
    }

    fn connect_as_child() -> Result<Self, std::io::Error> {
        todo!()
    }

    /// Check if both threads are still running
    pub fn is_alive(&self) -> bool {
        !self.threads.0.is_finished() && !self.threads.1.is_finished()
    }

    /// Get the current status of both threads and the pipe
    pub fn thread_status(&self) -> DuplexPipeStatus {
        let write_thread_finished = self.threads.0.is_finished();
        let read_thread_finished = self.threads.1.is_finished();
        DuplexPipeStatus {
            write_thread_finished,
            read_thread_finished,
            pipe_alive: !write_thread_finished && !read_thread_finished,
        }
    }

    /// Check for errors in background threads without blocking.
    /// Returns a summary of thread states and any errors discovered.
    pub fn check_errors(&self) -> String {
        let status = self.thread_status();
        let mut report = String::new();

        report.push_str("DuplexPipe Thread Status:\n");
        report.push_str(&format!("  Pipe alive: {}\n", status.pipe_alive));
        report.push_str(&format!(
            "  Write thread finished: {:#?}\n",
            status.write_thread_finished
        ));
        report.push_str(&format!(
            "  Read thread finished: {:#?}\n",
            status.read_thread_finished
        ));
        report.push_str(&format!(
            "  Outgoing queue length: {}\n",
            self.outgoing_queue_len()
        ));
        report.push_str(&format!(
            "  Incoming queue length: {}\n",
            self.incoming_queue_len()
        ));

        report
    }

    /// Get the length of the outgoing queue (commands waiting to be sent to subprocess)
    pub fn outgoing_queue_len(&self) -> usize {
        self.outgoing.len()
    }

    /// Get the length of the incoming queue (responses waiting to be received from subprocess)
    pub fn incoming_queue_len(&self) -> usize {
        self.incoming.len()
    }

    pub fn close(&self) {
        self.threads.0.abort();
        self.threads.1.abort();
    }

    pub fn incoming(&self) -> &Arc<Queue<Command>> {
        &self.incoming
    }

    pub fn outgoing(&self) -> &Arc<Queue<Command>> {
        &self.outgoing
    }
}

impl Transport<Command> for DuplexPipe {
    fn send_blocking(&self, data: Command) -> Result<(), TransportError> {
        self.outgoing.send_blocking(data)
    }

    fn send_batch_blocking(&self, data: Vec<Command>) -> Result<(), TransportError> {
        self.outgoing.send_batch_blocking(data)
    }

    fn recv_blocking(&self) -> Result<Command, TransportError> {
        self.incoming.recv_blocking()
    }

    fn recv_avaliable_blocking(&self) -> Result<Vec<Command>, TransportError> {
        self.incoming.recv_avaliable_blocking()
    }

    fn try_recv_blocking(&self) -> Result<Option<Command>, TransportError> {
        self.incoming.try_recv_blocking()
    }

    fn send(
        &self,
        data: Command,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<(), TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        self.outgoing.send(data)
    }

    fn send_batch(
        &self,
        data: Vec<Command>,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<(), TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        self.outgoing.send_batch(data)
    }

    fn recv(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<Command, TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        self.incoming.recv()
    }

    fn recv_avaliable(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<Vec<Command>, TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        self.incoming.recv_avaliable()
    }

    fn try_recv(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<Option<Command>, TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        self.incoming.try_recv()
    }
}
