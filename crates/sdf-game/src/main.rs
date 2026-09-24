use crate::{
    game::PlayerUpdate,
    net::{Broadcast, NetEvent},
};
use al_events::{register_event, EventHelpers, FormatId};
use std::io::{self, Write};

mod app;
mod game;
mod map;
mod net;
mod render;

const SERVER_ADDR: &str = "0.0.0.0:7878";
const CLIENT_ADDR: &str = "127.0.0.1:7878";

#[derive(Debug, Default)]
enum Mode {
    #[default]
    Client,
    Server,
}

fn main() {
    register_event!(NetEvent);
    register_event!(Broadcast);
    register_event!(PlayerUpdate);
    let f_id =
        al_events::register_format!(al_structures::serde_utils::formats::JsonFormat).unwrap();

    let mode = parse_mode_from_str(
        &std::env::args()
            .nth(1)
            .map(|s| s.to_lowercase())
            .unwrap_or_default(),
    )
    .unwrap_or_else(prompt_for_mode);
    match mode {
        Mode::Server => run_server(f_id),
        Mode::Client => run_client(f_id),
    }
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    pollster::block_on(future)
}

fn parse_mode_from_str(str: &str) -> Option<Mode> {
    match str {
        "client" | "c" => Some(Mode::Client),
        "server" | "s" => Some(Mode::Server),
        _ => None,
    }
}

fn prompt_for_mode() -> Mode {
    loop {
        println!("Run as [s]erver or [c]lient? ");
        io::stdout().flush().ok();
        let mut s = String::new();
        if io::stdin().read_line(&mut s).is_err() {
            eprintln!("stdin read failed, defaulting to client");
            return Mode::Client;
        }
        match parse_mode_from_str(&s.trim().to_lowercase()) {
            Some(mode) => return mode,
            None => println!("Please enter 's' or 'c'."),
        }
    }
}

fn parse_addr_from_str(str: String) -> Option<String> {
    if str.is_empty() {
        return Some(CLIENT_ADDR.to_string());
    }
    let validation = str.split(':').collect::<Vec<_>>();
    if validation.len() == 2 && validation[0].len() <= 39 && validation[1].len() <= 5 {
        Some(str)
    } else {
        None
    }
}

fn prompt_for_addr() -> String {
    loop {
        println!("Enter address to connect to [default is `127.0.0.1:7878`]: ");
        io::stdout().flush().ok();
        let mut s = String::new();
        if io::stdin().read_line(&mut s).is_err() {
            eprintln!("stdin read failed, defaulting to CLIENT_ADDR");
            return CLIENT_ADDR.to_string();
        }
        match parse_addr_from_str(s.trim().to_lowercase()) {
            Some(addr) => return addr,
            None => println!("Please enter address to connect to [empty for `127.0.0.1:7878`]: "),
        }
    }
}

fn run_client(format_id: FormatId) {
    let addr = prompt_for_addr();
    block_on(app::App::new(
        net::NetworkState::new_client(format_id, addr),
        std::time::Duration::from_millis(50),
        Default::default(),
        Default::default(),
    ))
    .run()
    .unwrap()
}

fn run_server(format_id: FormatId) {
    println!("Starting server on {SERVER_ADDR}");
    let net_state = net::NetworkState::new_server(format_id, SERVER_ADDR.to_string());
    let dispatcher = net::Dispatcher::new(
        net_state.state().clone(),
        Default::default(),
        Default::default(),
    );
    block_on(dispatcher.register_event(|ctx, _, update: PlayerUpdate| {
        Box::pin(async move {
            if let Some(src) = ctx.source() {
                let relay = PlayerUpdate::new(src, update.pos(), update.radius()).to_msg();
                if let Err(e) = ctx.net().broadcast(Some(vec![src]), relay) {
                    eprintln!("Server `PlayerUpdate` relay failed: {e}");
                }
            }
        })
    }))
    .unwrap();
    block_on(dispatcher.register_event(|_, state, evt: NetEvent| {
        Box::pin(async move {
            match evt {
                NetEvent::NoOp => {}
                NetEvent::Connected(id) => println!("Client {id} connected"),
                NetEvent::Disconnected(id) => {
                    let mut s = state.write().await;
                    if s.local_conn() == Some(id) {
                        s.set_local_conn(None);
                    }
                    s.remove_player(id);
                    println!("Client {id} disconnected");
                }
            }
        })
    }))
    .unwrap();
    loop {
        if let Err(e) = block_on(net_state.update(&dispatcher)) {
            eprintln!("Server update error: {e}");
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
