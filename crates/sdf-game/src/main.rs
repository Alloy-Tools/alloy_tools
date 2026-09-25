use crate::{
    game::{HealthUpdate, PlayerUpdate, ProjectileSpawn, Scoreboard, FIXED_DT, RESPAWN_DELAY},
    net::{Broadcast, NetEvent},
};
use al_events::{register_event, EventHelpers, FormatId};
use std::{
    io::{self, Write},
    time::Instant,
};

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
    Host,
}

fn main() {
    register_event!(NetEvent);
    register_event!(Broadcast);
    register_event!(PlayerUpdate);
    register_event!(ProjectileSpawn);
    register_event!(HealthUpdate);
    register_event!(Scoreboard);
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
        Mode::Host => run_host(f_id),
    }
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    pollster::block_on(future)
}

fn parse_mode_from_str(str: &str) -> Option<Mode> {
    match str {
        "client" | "c" => Some(Mode::Client),
        "server" | "s" => Some(Mode::Server),
        "host" | "h" => Some(Mode::Host),
        _ => None,
    }
}

fn prompt_for_mode() -> Mode {
    loop {
        println!("Run as [s]erver, [c]lient, or [h]ost? ");
        io::stdout().flush().ok();
        let mut s = String::new();
        if io::stdin().read_line(&mut s).is_err() {
            eprintln!("stdin read failed, defaulting to client");
            return Mode::Client;
        }
        match parse_mode_from_str(&s.trim().to_lowercase()) {
            Some(mode) => return mode,
            None => println!("Please enter 's', 'c', or 'h'."),
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
    println!("Connecting to server at '{addr}'");
    block_on(app::App::new(
        net::NetworkState::new_client(format_id, addr),
        std::time::Duration::from_millis(50),
        Default::default(),
        Default::default(),
    ))
    .run()
    .unwrap()
}

fn run_host(format_id: FormatId) {
    use std::process::{Command, Stdio};

    let exe = std::env::current_exe().expect("current exe");
    let mut child = Command::new(exe)
        .arg("server")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to spawn server subprocess");

    // wait for server to start accepting connections
    let addr = CLIENT_ADDR.to_string();
    let mut connected = false;
    for _ in 0..30 {
        if std::net::TcpStream::connect(&addr).is_ok() {
            connected = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    if !connected {
        eprintln!("Server subprocess didn't start listening in time.");
        let _ = child.kill();
        std::process::exit(1);
    }

    let result = block_on(app::App::new(
        net::NetworkState::new_client(format_id, addr),
        std::time::Duration::from_millis(50),
        Default::default(),
        Default::default(),
    ))
    .run();

    // Parent exit: kill the child so a stray process isn't left.
    let _ = child.kill();
    let _ = child.wait();
    result.unwrap();
}

fn run_server(format_id: FormatId) {
    println!("Starting server on {SERVER_ADDR}");
    let net_state = net::NetworkState::new_server(format_id, SERVER_ADDR.to_string());
    let dispatcher = net::Dispatcher::new(
        net_state.state().clone(),
        Default::default(),
        Default::default(),
    );
    block_on(
        dispatcher.register_event(|ctx, state, update: PlayerUpdate| {
            Box::pin(async move {
                let Some(src) = ctx.source() else { return };
                state
                    .write()
                    .await
                    .update_server_player(src, update.pos(), update.radius());
                let relay = PlayerUpdate::new(src, update.pos(), update.radius()).to_msg();
                if let Err(e) = ctx.net().broadcast(Some(vec![src]), relay) {
                    eprintln!("Server `PlayerUpdate` relay failed: {e}");
                }
            })
        }),
    )
    .unwrap();
    block_on(
        dispatcher.register_event(|ctx, state, spawn: ProjectileSpawn| {
            Box::pin(async move {
                let Some(src) = ctx.source() else { return };
                state
                    .write()
                    .await
                    .queue_server_projectile(src, spawn.pos(), spawn.dir());
                let relay = ProjectileSpawn::new(src, spawn.pos(), spawn.dir()).to_msg();
                if let Err(e) = ctx.net().broadcast(Some(vec![src]), relay) {
                    eprintln!("Server `ProjectileSpawn` relay failed: {e}");
                }
            })
        }),
    )
    .unwrap();
    block_on(dispatcher.register_event(|ctx, state, evt: NetEvent| {
        Box::pin(async move {
            match evt {
                NetEvent::NoOp | NetEvent::Welcome(_) => {}
                NetEvent::Connected(id) => {
                    println!("Client {id} connected");
                    state.write().await.ensure_server_player(id);
                    if let Err(e) = ctx.net().send_to(id, NetEvent::Welcome(id).to_msg()) {
                        eprintln!("Server `Welcome` send failed: {e}");
                    }
                }
                NetEvent::Disconnected(id) => {
                    let mut s = state.write().await;
                    if s.local_conn() == Some(id) {
                        s.set_local_conn(None);
                    }
                    s.remove_player(id);
                    s.remove_server_player(id);
                    println!("Client {id} disconnected");
                }
            }
        })
    }))
    .unwrap();

    let mut accumulator = 0.0f32;
    let mut last_tick = Instant::now();
    loop {
        if let Err(e) = block_on(net_state.update(&dispatcher)) {
            eprintln!("Server update error: {e}");
        }

        let now = Instant::now();
        let dt = (now - last_tick).as_secs_f32().min(0.25);
        last_tick = now;
        accumulator += dt;

        while accumulator >= FIXED_DT {
            let hits = block_on(net_state.state().write())
                .tick_server_projectiles(crate::map::map_sdf, FIXED_DT);

            for hit in hits {
                let (new_hp, scoreboard) = {
                    let mut s = block_on(net_state.state().write());
                    let Some(target) = s.server_player_mut(hit.target()) else {
                        continue;
                    };
                    target.hp = (target.hp - hit.damage()).max(0.);
                    if target.hp <= 0. {
                        target.deaths += 1;
                        target.respawn_at = Some(Instant::now() + RESPAWN_DELAY);
                        //TODO: credit killer?
                        /*if let Some(sh) = state.write().await.server_player_mut(shooter) {
                            sh.kills += 1;
                        }*/
                    }
                    (target.hp, s.server_scoreboard())
                };
                if let Err(e) = net_state
                    .handle()
                    .broadcast(None, HealthUpdate::new(hit.target(), new_hp).to_msg())
                {
                    eprintln!("Server failed to broadcast `HealthUpdate`: {e}");
                }
                if new_hp <= 0. {
                    if let Err(e) = net_state.handle().broadcast(None, scoreboard.to_msg()) {
                        eprintln!("Server failed to broadcast `Scoreboard`: {e}");
                    }
                }
            }
            accumulator -= FIXED_DT;
        }

        let respawned = block_on(net_state.state().write()).tick_respawns();
        for id in respawned {
            if let Err(e) = net_state
                .handle()
                .broadcast(None, HealthUpdate::new(id, crate::game::MAX_HP).to_msg())
            {
                eprintln!("Server failed to broadcast respawn `HealthUpdate`: {e}");
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}
