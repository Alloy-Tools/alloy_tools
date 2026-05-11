use al_core::Transport;
use al_pipe::DuplexPipe;
use std::process::Command;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread")]
async fn echo() {
    // Spawn pipe with 5-second timeout to catch hangs early
    let pipe = DuplexPipe::spawn_with_timeout(
        Command::new(env!("CARGO_BIN_EXE_echo")),
        Some(Duration::from_secs(5)),
    )
    .unwrap();

    // send pulse command
    pipe.send(al_core::Command::Pulse).await.unwrap();

    // recv echo
    let cmd = pipe.recv().await.unwrap();
    assert_eq!(cmd, al_core::Command::Pulse);
    assert_ne!(cmd, al_core::Command::Stop);

    // send command
    pipe.send(al_core::Command::Stop).await.unwrap();

    // recv echo
    let cmd = pipe.recv().await.unwrap();
    assert_eq!(cmd, al_core::Command::Stop);
    assert_ne!(cmd, al_core::Command::Pulse);

    pipe.close();
}
