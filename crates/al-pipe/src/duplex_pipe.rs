use al_core::{BinarySerde, Command, Queue, SerdeFormat, Transport, TransportError};
use std::{
    io::{Read, Write},
    process::{Child, Command as Process, Stdio},
    sync::Arc,
};

pub struct DuplexPipe {
    incoming: Arc<Queue<Command>>,
    outgoing: Arc<Queue<Command>>,
    threads: (
        tokio::task::JoinHandle<Result<(), String>>,
        tokio::task::JoinHandle<Result<(), String>>,
    ),
    child: Child,
}

impl DuplexPipe {
    pub fn spawn(mut command: Process) -> Result<Self, std::io::Error> {
        // Set up stdio pipes
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        let mut child = command.spawn()?;
        let pipe_in = child
            .stdin
            .take()
            .expect("Child should always have stdin on spawn");
        let pipe_out = child
            .stdout
            .take()
            .expect("Child should always have stdout on spawn");

        // Set up queues
        let queue_in = Arc::new(Queue::new());
        let queue_out = Arc::new(Queue::new());

        // spawn writer thread
        let write_queue = queue_in.clone();
        let write_thread: tokio::task::JoinHandle<Result<(), String>> = tokio::spawn(async move {
            let mut stdin = pipe_in;
            let serializer = BinarySerde;

            loop {
                tokio::task::yield_now().await;

                match write_queue.recv().await {
                    Ok(cmd) => {
                        let buf = serializer
                            .serialize_command(&cmd)
                            .map_err(|e| e.to_string())?;
                        if let Err(e) = stdin.write_all(&(buf.len() as u16).to_be_bytes()) {
                            Err(format!("Write error: {}", e))?;
                        }

                        if let Err(e) = stdin.write_all(&buf) {
                            Err(format!("Write error: {}", e))?;
                        }

                        let _ = stdin.flush();
                    }
                    Err(e) => Err(format!("{:?}", e))?,
                }
            }
        });

        // spawn reader thread
        let read_queue = queue_out.clone();
        let read_thread: tokio::task::JoinHandle<Result<(), String>> = tokio::spawn(async move {
            let mut stdout = pipe_out;
            let mut buf = [0u8; 1024];
            let mut len_bytes = [0u8; 2];
            let mut len: usize;
            let serializer = BinarySerde;

            while stdout.read_exact(&mut len_bytes).is_ok() {
                len = u16::from_be_bytes(len_bytes) as usize;

                stdout
                    .read_exact(&mut buf[..len])
                    .map_err(|e| e.to_string())?;

                if let Err(e) = read_queue
                    .send(
                        serializer
                            .deserialize_command(&buf[..len])
                            .map_err(|e| e.to_string())?,
                    )
                    .await
                {
                    Err(format!("{:?}", e))?;
                }
            }

            Ok(())
        });

        Ok(Self {
            incoming: queue_out,
            outgoing: queue_in,
            threads: (write_thread, read_thread),
            child,
        })
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

    pub async fn send(&self, cmd: Command) -> Result<(), TransportError> {
        self.outgoing.send(cmd).await
    }

    pub async fn recv(&self) -> Result<Command, TransportError> {
        self.incoming.recv().await
    }
}
