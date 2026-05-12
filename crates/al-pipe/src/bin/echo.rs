use al_core::Transport;
use al_pipe::{DuplexPipe, DuplexPipeError};
use std::time::Duration;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), DuplexPipeError> {
    let pipe = DuplexPipe::connect_as_child_with_timeout(Some(Duration::from_secs(5)))?;

    eprintln!("Spawned pipes");

    match pipe.recv().await {
        Ok(cmd) => {
            //eprintln!("Echoing data: {:?}", cmd);
            pipe.send(cmd).await?;
        }
        Err(e) => {
            eprintln!("Error: {:?}", e)
        }
    }

    match pipe.recv().await {
        Ok(cmd) => {
            //eprintln!("Echoing data: {:?}", cmd);
            pipe.send(cmd).await?;
        }
        Err(e) => {
            eprintln!("Error: {:?}", e)
        }
    }

    eprintln!("done");

    pipe.close();

    Ok(())
}
