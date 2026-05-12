use al_core::{BinarySerde, Command, Queue, SerdeFormat, Transport, TransportError};
use std::{
    io::{Read, Write},
    process::{Child, Command as Process, Stdio},
    sync::{Arc, Mutex},
    time::Duration,
};

/// Status of individual reader/writer threads
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DuplexPipeStatus {
    pub write_thread_finished: bool,
    pub read_thread_finished: bool,
    pub pipe_alive: bool,
}

/// Errors that can occur while creating or using a `DuplexPipe`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DuplexPipeError {
    ChildSpawnFailed(String),
    ChildStdioMissing,
    MessageTooLarge(usize),
    WriteTimeout(Duration),
    ReadTimeout(Duration),
    IOError(String),
    SerdeError(String),
    TransportError(String),
    UnexpectedEof,
}

impl std::fmt::Display for DuplexPipeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DuplexPipeError::ChildSpawnFailed(msg) => write!(f, "Child spawn failed: {msg}"),
            DuplexPipeError::ChildStdioMissing => write!(f, "Child stdio handle missing"),
            DuplexPipeError::MessageTooLarge(size) => {
                write!(
                    f,
                    "Serialized command size {size} exceeds u16 message limit"
                )
            }
            DuplexPipeError::WriteTimeout(duration) => {
                write!(f, "Write timeout after {duration:?}")
            }
            DuplexPipeError::ReadTimeout(duration) => write!(f, "Read timeout after {duration:?}"),
            DuplexPipeError::IOError(msg) => write!(f, "I/O error: {msg}"),
            DuplexPipeError::SerdeError(msg) => write!(f, "Serde error: {msg}"),
            DuplexPipeError::TransportError(msg) => write!(f, "Channel error: {msg}"),
            DuplexPipeError::UnexpectedEof => write!(f, "Unexpected EOF reached on pipe"),
        }
    }
}

impl std::error::Error for DuplexPipeError {}

impl From<std::io::Error> for DuplexPipeError {
    fn from(e: std::io::Error) -> Self {
        DuplexPipeError::IOError(e.to_string())
    }
}

impl From<TransportError> for DuplexPipeError {
    fn from(e: TransportError) -> Self {
        DuplexPipeError::TransportError(format!("{e:?}"))
    }
}

#[derive(Debug)]
pub struct DuplexPipe {
    incoming: Arc<Queue<Command>>,
    outgoing: Arc<Queue<Command>>,
    threads: (
        tokio::task::JoinHandle<Result<(), DuplexPipeError>>,
        tokio::task::JoinHandle<Result<(), DuplexPipeError>>,
    ),
    #[allow(unused)]
    child: Option<Child>,
    last_error: Arc<Mutex<Option<DuplexPipeError>>>,
}

impl DuplexPipe {
    pub fn spawn(command: Process) -> Result<Self, DuplexPipeError> {
        Self::spawn_with_timeout(command, None)
    }

    pub fn spawn_with_timeout(
        mut command: Process,
        timeout: Option<Duration>,
    ) -> Result<Self, DuplexPipeError> {
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        let mut child = command
            .spawn()
            .map_err(|e| DuplexPipeError::ChildSpawnFailed(e.to_string()))?;
        let pipe_child_in = child
            .stdin
            .take()
            .ok_or(DuplexPipeError::ChildStdioMissing)?;
        let pipe_child_out = child
            .stdout
            .take()
            .ok_or(DuplexPipeError::ChildStdioMissing)?;

        let queue_child_in = Arc::new(Queue::new());
        let queue_child_out = Arc::new(Queue::new());
        let last_error = Arc::new(Mutex::new(None));

        let write_last_error = last_error.clone();
        let write_queue = queue_child_in.clone();
        let write_thread: tokio::task::JoinHandle<Result<(), DuplexPipeError>> =
            tokio::spawn(async move {
                let mut stdin = pipe_child_in;
                let serializer = BinarySerde;
                loop {
                    let cmd = write_queue
                        .recv()
                        .await
                        .map_err(|e| DuplexPipeError::TransportError(format!("{e:?}")))?;

                    let buf = serializer
                        .serialize_command(&cmd)
                        .map_err(|e| DuplexPipeError::SerdeError(e.to_string()))?;

                    if buf.len() > u16::MAX as usize {
                        return Err(DuplexPipeError::MessageTooLarge(buf.len()));
                    }

                    let write_op = async {
                        tokio::task::block_in_place(|| {
                            stdin.write_all(&(buf.len() as u16).to_be_bytes())
                        })
                        .map_err(|e| DuplexPipeError::IOError(e.to_string()))?;
                        tokio::task::block_in_place(|| stdin.write_all(&buf))
                            .map_err(|e| DuplexPipeError::IOError(e.to_string()))?;
                        tokio::task::block_in_place(|| stdin.flush())
                            .map_err(|e| DuplexPipeError::IOError(e.to_string()))?;
                        Ok::<(), DuplexPipeError>(())
                    };

                    match timeout {
                        Some(duration) => match tokio::time::timeout(duration, write_op).await {
                            Ok(result) => result?,
                            Err(_) => {
                                return Err(DuplexPipeError::WriteTimeout(duration));
                            }
                        },
                        None => write_op.await?,
                    }
                }
            });

        let read_last_error = last_error.clone();
        let read_queue = queue_child_out.clone();
        let read_thread: tokio::task::JoinHandle<Result<(), DuplexPipeError>> =
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let mut len_bytes = [0u8; 2];
                let mut stdout = pipe_child_out;
                let serializer = BinarySerde;

                loop {
                    match tokio::task::block_in_place(|| stdout.read_exact(&mut len_bytes)) {
                        Ok(()) => {
                            let len = u16::from_be_bytes(len_bytes) as usize;
                            let payload = if len <= buf.len() {
                                tokio::task::block_in_place(|| stdout.read_exact(&mut buf[..len]))
                                    .map_err(|e| DuplexPipeError::IOError(e.to_string()))?;
                                buf[..len].to_vec()
                            } else {
                                let mut payload = vec![0u8; len];
                                tokio::task::block_in_place(|| stdout.read_exact(&mut payload))
                                    .map_err(|e| DuplexPipeError::IOError(e.to_string()))?;
                                payload
                            };

                            let cmd = serializer
                                .deserialize_command(&payload)
                                .map_err(|e| DuplexPipeError::SerdeError(e.to_string()))?;

                            read_queue
                                .send(cmd)
                                .await
                                .map_err(|e| DuplexPipeError::TransportError(format!("{e:?}")))?;
                        }
                        Err(e) => {
                            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                                if let Ok(mut guard) = read_last_error.lock() {
                                    *guard = Some(DuplexPipeError::UnexpectedEof);
                                }
                            }
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
            child: Some(child),
            last_error,
        })
    }

    pub fn connect_as_child() -> Result<Self, DuplexPipeError> {
        Self::connect_as_child_with_timeout(None)
    }

    pub fn connect_as_child_with_timeout(
        timeout: Option<Duration>,
    ) -> Result<Self, DuplexPipeError> {
        let queue_child_in = Arc::new(Queue::new());
        let queue_child_out = Arc::new(Queue::new());
        let last_error = Arc::new(Mutex::new(None));

        let read_last_error = last_error.clone();
        let read_queue = queue_child_in.clone();
        let read_thread: tokio::task::JoinHandle<Result<(), DuplexPipeError>> =
            tokio::spawn(async move {
                let mut stdin = std::io::stdin();
                let mut len_bytes = [0u8; 2];
                let mut payload = vec![0u8; 1024];
                let serializer = BinarySerde;

                loop {
                    let read_len = async {
                        tokio::task::block_in_place(|| {
                            std::io::Read::read_exact(&mut stdin, &mut len_bytes)
                        })
                    };

                    match timeout {
                        Some(duration) => {
                            if tokio::time::timeout(duration, read_len).await.is_err()
                            {
                                break;
                            }
                        }
                        None => {
                            if std::io::Read::read_exact(&mut stdin, &mut len_bytes).is_err() {
                                break;
                            }
                        }
                    }

                    let len = u16::from_be_bytes(len_bytes) as usize;
                    // Reuse buffer, grow only if needed
                    if payload.capacity() < len {
                        payload.resize(len, 0);
                    }

                    let read_payload = async {
                        tokio::task::block_in_place(|| {
                            std::io::Read::read_exact(&mut stdin, &mut payload[..len])
                        })
                    };

                    match timeout {
                        Some(duration) => {
                            if tokio::time::timeout(duration, read_payload).await.is_err()
                            {
                                break;
                            }
                        }
                        None => {
                            if std::io::Read::read_exact(&mut stdin, &mut payload[..len]).is_err() {
                                break;
                            }
                        }
                    }

                    let cmd = serializer
                        .deserialize_command(&payload[..len])
                        .map_err(|e| DuplexPipeError::SerdeError(e.to_string()))?;

                    read_queue
                        .send(cmd)
                        .await
                        .map_err(|e| DuplexPipeError::TransportError(format!("{e:?}")))?;
                }
                Ok(())
            });

        // Child writer: reads from outgoing queue, writes responses to stdout
        let write_last_error = last_error.clone();
        let write_queue = queue_child_out.clone();
        let write_thread: tokio::task::JoinHandle<Result<(), DuplexPipeError>> =
            tokio::spawn(async move {
                let mut stdout = std::io::stdout();
                let serializer = BinarySerde;

                loop {
                    let cmd = write_queue
                        .recv()
                        .await
                        .map_err(|e| DuplexPipeError::TransportError(format!("{e:?}")))?;

                    let buf = serializer
                        .serialize_command(&cmd)
                        .map_err(|e| DuplexPipeError::SerdeError(e.to_string()))?;

                    if buf.len() > u16::MAX as usize {
                        return Err(DuplexPipeError::MessageTooLarge(buf.len()));
                    }

                    let write_op = async {
                        tokio::task::block_in_place(|| {
                            std::io::Write::write_all(
                                &mut stdout,
                                &(buf.len() as u16).to_be_bytes(),
                            )
                            .map_err(|e| DuplexPipeError::IOError(e.to_string()))?;
                            std::io::Write::write_all(&mut stdout, &buf)
                                .map_err(|e| DuplexPipeError::IOError(e.to_string()))?;
                            std::io::Write::flush(&mut stdout)
                                .map_err(|e| DuplexPipeError::IOError(e.to_string()))?;
                            Ok::<(), DuplexPipeError>(())
                        })
                    };

                    match timeout {
                        Some(duration) => match tokio::time::timeout(duration, write_op).await {
                            Ok(result) => result.map_err(|e| {
                                if let Ok(mut guard) = write_last_error.lock() {
                                    *guard = Some(e.clone());
                                }
                                e
                            })?,
                            Err(_) => {
                                return Err(DuplexPipeError::WriteTimeout(duration));
                            }
                        },
                        None => write_op.await.map_err(|e| {
                            if let Ok(mut guard) = write_last_error.lock() {
                                *guard = Some(e.clone());
                            }
                            e
                        })?,
                    }
                }
            });

        Ok(Self {
            incoming: queue_child_in,
            outgoing: queue_child_out,
            threads: (write_thread, read_thread),
            child: None,
            last_error,
        })
    }

    pub fn is_alive(&self) -> bool {
        !self.threads.0.is_finished() && !self.threads.1.is_finished()
    }

    pub fn thread_status(&self) -> DuplexPipeStatus {
        let write_thread_finished = self.threads.0.is_finished();
        let read_thread_finished = self.threads.1.is_finished();
        DuplexPipeStatus {
            write_thread_finished,
            read_thread_finished,
            pipe_alive: !write_thread_finished && !read_thread_finished,
        }
    }

    pub fn last_error(&self) -> Option<DuplexPipeError> {
        self.last_error.lock().ok().and_then(|guard| guard.clone())
    }

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
        if let Some(error) = self.last_error() {
            report.push_str(&format!("  Last error: {error}\n"));
        }

        report
    }

    pub fn outgoing_queue_len(&self) -> usize {
        self.outgoing.len()
    }

    pub fn incoming_queue_len(&self) -> usize {
        self.incoming.len()
    }

    pub fn close(&self) {
        self.threads.0.abort();
        self.threads.1.abort();

        //TODO: abort child process
    }

    pub fn incoming(&self) -> &Arc<Queue<Command>> {
        &self.incoming
    }

    pub fn outgoing(&self) -> &Arc<Queue<Command>> {
        &self.outgoing
    }
}

impl Drop for DuplexPipe {
    fn drop(&mut self) {
        self.close();
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
