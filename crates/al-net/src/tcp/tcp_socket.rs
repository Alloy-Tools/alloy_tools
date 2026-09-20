use std::sync::Arc;
use al_secure::noise::MAX_MSG_BYTE_LEN;
use al_structures::cancellation::CancellationToken;
use al_transport::transports::BoundaryQueue;

//REVIEW: Include timeout
pub struct TcpSocket {
    to_wire: Arc<BoundaryQueue<Vec<u8>>>,
    from_wire: Arc<BoundaryQueue<Vec<u8>>>,
}

impl TcpSocket {
    pub fn new(
        to_wire: Arc<BoundaryQueue<Vec<u8>>>,
        from_wire: Arc<BoundaryQueue<Vec<u8>>>,
    ) -> Self {
        Self { to_wire, from_wire }
    }

    pub async fn run(self, stream: tokio::net::TcpStream) -> std::io::Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let (mut reader, mut writer) = stream.into_split();
        let to_wire = self.to_wire.clone();
        let from_wire = self.from_wire.clone();

        let read_task = tokio::spawn(async move {
            let mut buf = Box::new([0u8; MAX_MSG_BYTE_LEN]);
            loop {
                match reader.read(buf.as_mut_slice()).await {
                    Ok(0) | Err(_) => return,
                    Ok(n) => {
                        if from_wire.send(buf[..n].to_vec()).is_err() {
                            return;
                        }
                    }
                }
            }
        });

        loop {
            let data = match to_wire.recv().await {
                Ok(d) => d,
                Err(_) => break,
            };
            if writer.write_all(&data).await.is_err() {
                break;
            }
        }

        read_task.abort();
        Ok(())
    }

    pub async fn connect<A: tokio::net::ToSocketAddrs>(self, addr: A) -> std::io::Result<()> {
        let stream = tokio::net::TcpStream::connect(addr).await?;
        self.run(stream).await
    }

    pub async fn run_with_cancel(self, stream: tokio::net::TcpStream, token: CancellationToken) -> std::io::Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let (mut reader, mut writer) = stream.into_split();
        let to_wire = self.to_wire.clone();
        let from_wire = self.from_wire.clone();

        let read_token = token.clone();
        let read_task = tokio::spawn(async move {
            let mut buf = Box::new([0u8; MAX_MSG_BYTE_LEN]);
            loop {
                let n = tokio::select! {
                    biased;
                    _ = read_token.cancelled() => return,
                    result = reader.read(buf.as_mut_slice()) => match result {
                        Ok(0) | Err(_) => return,
                        Ok(n) => n,
                    }
                };
                if from_wire.send(buf[..n].to_vec()).is_err() {
                    return;
                }
            }
        });

        loop {
            let data = tokio::select! {
                biased;
                _ = token.cancelled() => break,
                data = to_wire.recv() => match data {
                    Ok(d) => d,
                    Err(_) => break,
                }
            };
            if writer.write_all(&data).await.is_err() {
                break;
            }
        }

        read_task.abort();
        Ok(())
    }

    pub async fn connect_with_cancel<A: tokio::net::ToSocketAddrs>(self, addr: A, token: CancellationToken) -> std::io::Result<()> {
        let stream = tokio::select! {
            biased;
            _ = token.cancelled() => return Ok(()),
            result = tokio::net::TcpStream::connect(addr) => result?,
        };
        self.run_with_cancel(stream, token).await
    }
}
