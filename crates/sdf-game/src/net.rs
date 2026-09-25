use al_events::{DynMessage, EventHelpers, FormatId};
use al_net::{
    message_dispatcher::{DynMessageDispatcher, DynMessageKey},
    tcp::{Tcp, TcpError, TcpSocket},
};
use al_structures::{
    cancellation::CancellationToken,
    collections::storage::{
        utils::{indexed::IndexedStorage, keyed::KeyedStorage},
        CowStorage,
    },
};
use al_transport::{
    dispatcher::Handler,
    transports::{BoundaryQueue, FallibleSink},
    Driver, DriverError, TransportError, TransportId,
};
use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
};
use tokio::{
    sync::{Mutex, RwLock},
    task::JoinSet,
};

use crate::game::{
    ScoreEntry, Scoreboard, DAMAGE_PER_HIT, MAX_HP, PROJECTILE_LIFETIME, PROJECTILE_RADIUS,
    PROJECTILE_SPEED,
};

pub type Payload = Vec<u8>;
pub type TaggedBytes = (ConnId, Payload);
pub type TaggedMsg = (ConnId, DynMessage);

pub type ConnId = u64;
pub type TransportIdStorage = HashMap<ConnId, TransportId>;
pub type ConnIdStorage = HashMap<TransportId, ConnId>;
pub type LiveStorage = Vec<ConnId>;
pub type NetTask =
    NetworkTask<al_crypto::Monotonic, TransportIdStorage, ConnIdStorage, LiveStorage>;

pub type MessageHandler = Handler<DynMessage, NetContext, NetStateRef>;
pub type AnyStore = CowStorage<Vec<MessageHandler>>;
pub type KeyedStore = CowStorage<HashMap<DynMessageKey, HandlerStore>>;
pub type HandlerStore = Vec<MessageHandler>;
pub type Dispatcher =
    DynMessageDispatcher<NetContext, NetStateRef, AnyStore, KeyedStore, HandlerStore>;

#[al_events::event]
pub enum NetEvent {
    #[default]
    NoOp,
    Connected(ConnId),
    Disconnected(ConnId),
    Welcome(ConnId),
}

#[al_events::event]
pub struct Broadcast {
    except: Option<Vec<ConnId>>,
    msg: Vec<u8>,
}

impl Broadcast {
    pub fn new(except: Option<Vec<ConnId>>, msg: Vec<u8>) -> Self {
        Self { except, msg }
    }
}

#[derive(Debug, Clone)]
pub struct RemotePlayer {
    pub conn_id: ConnId,
    pub pos: [f32; 2],
    pub radius: f32,
}

impl RemotePlayer {
    pub fn new(conn_id: ConnId, pos: [f32; 2], radius: f32) -> Self {
        Self {
            conn_id,
            pos,
            radius,
        }
    }
}

pub struct ServerPlayer {
    pub pos: [f32; 2],
    pub radius: f32,
    pub hp: f32,
    pub respawn_at: Option<std::time::Instant>,
    pub kills: u32,
    pub deaths: u32,
}

pub struct ServerProjectile {
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub life: f32,
    pub owner: ConnId,
}

pub struct Hit {
    target: ConnId,
    damage: f32,
}
impl Hit {
    pub fn new(target: ConnId, damage: f32) -> Self {
        Self { target, damage }
    }
    pub fn target(&self) -> ConnId {
        self.target
    }
    pub fn damage(&self) -> f32 {
        self.damage
    }
}

pub type NetStateRef = Arc<RwLock<NetState>>;
pub struct NetState {
    /// The single connection this app owns (client mode only).
    local_conn: Option<ConnId>,
    /// Server assigned ConnId
    server_id: Option<ConnId>,
    players: HashMap<ConnId, RemotePlayer>,
    pending_projectiles: Vec<(ConnId, [f32; 2], [f32; 2])>,
    pending_health: Vec<(ConnId, f32)>,
    pending_scoreboard: Option<Scoreboard>,
    server_players: HashMap<ConnId, ServerPlayer>,
    server_projectiles: Vec<ServerProjectile>,
}

impl NetState {
    pub fn new() -> Self {
        Self {
            local_conn: None,
            server_id: None,
            players: HashMap::new(),
            pending_projectiles: Vec::new(),
            pending_health: Vec::new(),
            pending_scoreboard: None,
            server_players: HashMap::new(),
            server_projectiles: Vec::new(),
        }
    }

    pub fn server_id(&self) -> Option<ConnId> {
        self.server_id
    }
    pub fn set_server_id(&mut self, id: Option<ConnId>) {
        self.server_id = id
    }

    pub fn local_conn(&self) -> Option<ConnId> {
        self.local_conn
    }
    pub fn set_local_conn(&mut self, id: Option<ConnId>) {
        self.local_conn = id
    }

    pub fn players(&self) -> impl Iterator<Item = &RemotePlayer> {
        self.players.values()
    }

    pub fn upsert_player(&mut self, conn_id: ConnId, pos: [f32; 2], radius: f32) {
        self.players
            .insert(conn_id, RemotePlayer::new(conn_id, pos, radius));
    }

    pub fn remove_player(&mut self, conn_id: ConnId) {
        self.players.remove(&conn_id);
    }

    pub fn queue_projectile(&mut self, owner: ConnId, pos: [f32; 2], dir: [f32; 2]) {
        self.pending_projectiles.push((owner, pos, dir));
    }

    pub fn drain_projectiles(&mut self) -> Vec<(ConnId, [f32; 2], [f32; 2])> {
        std::mem::take(&mut self.pending_projectiles)
    }

    pub fn queue_health(&mut self, conn_id: ConnId, hp: f32) {
        self.pending_health.push((conn_id, hp));
    }

    pub fn drain_health(&mut self) -> Vec<(ConnId, f32)> {
        std::mem::take(&mut self.pending_health)
    }

    pub fn set_scoreboard(&mut self, scoreboard: Scoreboard) {
        self.pending_scoreboard = Some(scoreboard);
    }

    pub fn take_scoreboard(&mut self) -> Option<Scoreboard> {
        self.pending_scoreboard.take()
    }

    pub fn server_player_mut(&mut self, id: ConnId) -> Option<&mut ServerPlayer> {
        self.server_players.get_mut(&id)
    }

    pub fn update_server_player(&mut self, id: ConnId, pos: [f32; 2], radius: f32) {
        if let Some(p) = self.server_players.get_mut(&id) {
            p.pos = pos;
            p.radius = radius;
        }
    }

    pub fn ensure_server_player(&mut self, id: ConnId) -> &mut ServerPlayer {
        self.server_players
            .entry(id)
            .or_insert_with(|| ServerPlayer {
                pos: [0., 0.],
                radius: 0.05,
                hp: crate::game::MAX_HP,
                respawn_at: None,
                kills: 0,
                deaths: 0,
            })
    }

    pub fn remove_server_player(&mut self, id: ConnId) {
        self.server_players.remove(&id);
    }

    pub fn server_scoreboard(&self) -> Scoreboard {
        Scoreboard::new(
            self.server_players
                .iter()
                .map(|(id, sp)| ScoreEntry {
                    conn_id: *id,
                    kills: sp.kills,
                    deaths: sp.deaths,
                })
                .collect(),
        )
    }

    pub fn tick_respawns(&mut self) -> Vec<ConnId> {
        let now = std::time::Instant::now();
        let mut ready = Vec::new();
        for (id, sp) in self.server_players.iter_mut() {
            if let Some(t) = sp.respawn_at {
                if now >= t {
                    sp.hp = MAX_HP;
                    sp.respawn_at = None;
                    ready.push(*id);
                }
            }
        }
        ready
    }

    pub fn queue_server_projectile(&mut self, owner: ConnId, pos: [f32; 2], dir: [f32; 2]) {
        self.server_projectiles.push(ServerProjectile {
            pos,
            vel: [dir[0] * PROJECTILE_SPEED, dir[1] * PROJECTILE_SPEED],
            life: PROJECTILE_LIFETIME,
            owner,
        });
    }

    pub fn tick_server_projectiles(&mut self, map: impl Fn(f32, f32) -> f32, dt: f32) -> Vec<Hit> {
        let mut hits = Vec::new();
        let mut alive = Vec::with_capacity(self.server_projectiles.len());

        for mut proj in std::mem::take(&mut self.server_projectiles) {
            proj.life -= dt;
            if proj.life <= 0. {
                continue;
            }

            let step_dist = (proj.vel[0] * proj.vel[0] + proj.vel[1] * proj.vel[1]).sqrt() * dt;
            let steps = (step_dist / (PROJECTILE_RADIUS * 0.9)).ceil().max(1.) as u32;
            let step_dt = dt / steps as f32;

            let mut died = false;
            'outer: for _ in 0..steps {
                proj.pos[0] += proj.vel[0] * step_dt;
                proj.pos[1] += proj.vel[1] * step_dt;

                // Map hit
                if map(proj.pos[0], proj.pos[1]) < PROJECTILE_RADIUS {
                    died = true;
                    break;
                }

                // Player hit
                for (id, p) in self.server_players.iter() {
                    if *id == proj.owner || p.hp <= 0. {
                        continue;
                    }

                    let dx = proj.pos[0] - p.pos[0];
                    let dy = proj.pos[1] - p.pos[1];
                    let r = PROJECTILE_RADIUS + p.radius;
                    if dx * dx + dy * dy < r * r {
                        hits.push(Hit::new(*id, DAMAGE_PER_HIT));
                        died = true;
                        break 'outer;
                    }
                }
            }
            if !died {
                alive.push(proj);
            }
        }
        self.server_projectiles = alive;
        hits
    }
}

#[derive(Clone)]
pub struct NetContext {
    /// `Some(conn_id)` if this message came from a peer, `None` if local.
    source: Option<ConnId>,
    net: Arc<NetworkHandle>,
}

impl NetContext {
    pub fn new(source: Option<ConnId>, net: Arc<NetworkHandle>) -> Self {
        Self { source, net }
    }

    pub fn source(&self) -> Option<ConnId> {
        self.source
    }
    pub fn net(&self) -> &Arc<NetworkHandle> {
        &self.net
    }
}

pub struct NetworkState {
    handle: Arc<NetworkHandle>,
    _task: std::thread::JoinHandle<()>,
    state: NetStateRef,
}

impl NetworkState {
    pub fn new_client(format_id: FormatId, addr: String) -> Self {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let state = Arc::new(RwLock::new(NetState::new()));
        let (handle, mut task) = NetworkHandle::new(format_id, rt.handle().clone());

        let net_task = std::thread::spawn(move || {
            rt.block_on(async move {
                let stream = match tokio::net::TcpStream::connect(&addr).await {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Could not connect to '{addr}': {e}");
                        std::process::exit(1);
                    }
                };
                let (tcp, socket) = al_net::tcp::Tcp::<al_crypto::Monotonic>::new_initiator(
                    al_secure::noise::handshake_pattern::HandshakePattern::NN,
                    Vec::new(),
                    None,
                    None,
                )
                .unwrap();
                task.add_tcp(tcp, socket, stream).await.unwrap();
                task.run(None).await;
            });
        });

        Self {
            handle: Arc::new(handle),
            _task: net_task,
            state,
        }
    }

    pub fn new_server(format_id: FormatId, addr: String) -> Self {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let state = Arc::new(RwLock::new(NetState::new()));
        let (handle, task) = NetworkHandle::new(format_id, rt.handle().clone());
        let handle = Arc::new(handle);

        let net_handle = handle.clone();
        let net_task = std::thread::spawn(move || {
            rt.block_on(async move {
                let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
                net_handle.mark_server();
                task.run(Some(listener)).await;
            });
        });

        Self {
            handle,
            _task: net_task,
            state,
        }
    }

    pub fn handle(&self) -> &Arc<NetworkHandle> {
        &self.handle
    }

    pub fn state(&self) -> &NetStateRef {
        &self.state
    }

    pub async fn local_conn_id(&self) -> Option<ConnId> {
        self.state.read().await.local_conn.clone()
    }

    pub async fn server_id(&self) -> Option<ConnId> {
        self.state.read().await.server_id.clone()
    }

    pub async fn update(
        &self,
        dispatcher: &Dispatcher,
    ) -> Result<(), al_transport::dispatcher::DispatcherError> {
        for evt in self.handle.events.recv_available().unwrap_or_default() {
            dispatcher
                .dispatch(NetContext::new(None, self.handle.clone()), evt.to_msg())
                .await?
        }

        for (conn_id, msg) in self.handle.app_in.recv_available().unwrap_or_default() {
            dispatcher
                .dispatch(NetContext::new(Some(conn_id), self.handle.clone()), msg)
                .await?
        }
        Ok(())
    }
}

pub struct NetworkHandle {
    format_id: al_events::FormatId,
    events: Arc<BoundaryQueue<NetEvent>>,
    app_out: Arc<BoundaryQueue<TaggedMsg>>,
    app_in: Arc<BoundaryQueue<TaggedMsg>>,
    _splices: Vec<al_transport::splice::SpliceHandle>,
    _token: CancellationToken,
    mode: AtomicBool,
}

impl NetworkHandle {
    pub fn new(format_id: al_events::FormatId, rt: tokio::runtime::Handle) -> (Self, NetTask) {
        let token = CancellationToken::new();
        let events = BoundaryQueue::new();
        let app_out = BoundaryQueue::new();
        let app_in = BoundaryQueue::new();
        let net_out = BoundaryQueue::new();
        let net_in = BoundaryQueue::new();

        let out_handle = al_transport::splice::splice_async(
            app_out.clone(),
            net_in.clone(),
            move |(id, msg): (ConnId, DynMessage)| {
                let mut bytes = Vec::new();
                msg.to_format(format_id, &mut bytes)
                    .expect("DynMessage serialization");
                (id, bytes)
            },
            |f| {
                rt.spawn(f);
            },
            al_transport::splice::panic_on_error,
        );

        let in_handle = al_transport::splice::splice_async(
            net_out.clone(),
            app_in.clone(),
            move |(id, bytes): (ConnId, Payload)| {
                (
                    id,
                    DynMessage::from_format_slice(&bytes).expect("DynMessage deserialization"),
                )
            },
            |f| {
                rt.spawn(f);
            },
            al_transport::splice::panic_on_error,
        );

        let task_events = events.clone();
        let task_token = token.clone();
        (
            Self {
                format_id,
                events,
                app_out,
                app_in,
                _splices: vec![out_handle, in_handle],
                _token: token,
                mode: AtomicBool::new(false),
            },
            NetworkTask::new(net_in, net_out, task_events, task_token),
        )
    }

    pub fn send_to(&self, conn_id: ConnId, msg: DynMessage) -> Result<(), TcpError> {
        self.app_out
            .send((conn_id, msg))
            .map_err(|_| TcpError::WireQueueClosed)
    }

    pub fn broadcast(&self, except: Option<Vec<ConnId>>, msg: DynMessage) -> Result<(), TcpError> {
        let mut bytes = Vec::new();
        msg.to_format(self.format_id, &mut bytes)?;
        self.app_out
            .send((ConnId::MAX, Broadcast::new(except, bytes).to_msg()))
            .map_err(|_| TcpError::WireQueueClosed)
    }

    pub fn mark_server(&self) {
        self.mode.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

pub struct NetworkTask<
    N: al_crypto::NonceTrait,
    M: KeyedStorage<ConnId, TransportId>,
    R: KeyedStorage<TransportId, ConnId>,
    L: IndexedStorage<ConnId>,
> {
    driver: Arc<Mutex<Driver<Payload>>>,
    events: Arc<BoundaryQueue<NetEvent>>,
    net_in: Arc<BoundaryQueue<TaggedBytes>>,
    net_out: Arc<BoundaryQueue<TaggedBytes>>,
    token: CancellationToken,
    conn_map: M,
    tcp_to_conn: R,
    live: Arc<tokio::sync::RwLock<L>>,
    next_conn_id: ConnId,
    socket_tasks: JoinSet<()>,
    _phantom: std::marker::PhantomData<N>,
}

impl<
        N: al_crypto::NonceTrait,
        M: KeyedStorage<ConnId, TransportId>,
        R: KeyedStorage<TransportId, ConnId>,
        L: IndexedStorage<ConnId>,
    > NetworkTask<N, M, R, L>
{
    pub fn new(
        net_in: Arc<BoundaryQueue<TaggedBytes>>,
        net_out: Arc<BoundaryQueue<TaggedBytes>>,
        events: Arc<BoundaryQueue<NetEvent>>,
        token: CancellationToken,
    ) -> Self {
        Self {
            driver: Arc::new(Mutex::new(Driver::new())),
            events,
            net_in,
            net_out,
            token,
            conn_map: M::new(),
            tcp_to_conn: R::new(),
            live: Arc::new(tokio::sync::RwLock::new(L::new())),
            next_conn_id: 0,
            socket_tasks: JoinSet::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn add_tcp(
        &mut self,
        tcp: Tcp<N>,
        socket: TcpSocket,
        stream: tokio::net::TcpStream,
    ) -> Result<ConnId, DriverError> {
        let conn_id = self.next_conn_id;
        self.next_conn_id += 1;

        let tcp_id = {
            let mut driver = self.driver.lock().await;
            let tcp_id = driver.add_transport(tcp);

            let out = self.net_out.clone();
            let sink_id = driver.add_transport(FallibleSink::new(move |bytes: Payload| {
                out.send((conn_id, bytes))
                    .map_err(|_| al_transport::Backpressure::Closed)
            }));
            driver.connect(tcp_id, sink_id)?;
            tcp_id
        };
        self.conn_map
            .insert(conn_id, tcp_id)
            .map_err(|e| DriverError::Transport(TransportError::Custom(Box::new(e))))?;
        self.tcp_to_conn
            .insert(tcp_id, conn_id)
            .map_err(|e| DriverError::Transport(TransportError::Custom(Box::new(e))))?;
        self.live
            .write()
            .await
            .push(conn_id)
            .map_err(|e| DriverError::Transport(TransportError::Custom(Box::new(e))))?;

        let conn_token = self.token.clone();
        self.socket_tasks.spawn(async move {
            if let Err(e) = socket.run_with_cancel(stream, conn_token).await {
                eprintln!("Chat driver socket from `add_tcp` failed: {e}")
            }
        });

        // Emit the connection
        if let Err(e) = self.events.send(NetEvent::Connected(conn_id)) {
            eprintln!("Chat `add_tcp` events `send` error: {e}");
        }
        Ok(conn_id)
    }

    pub async fn run(mut self, listener: Option<tokio::net::TcpListener>) {
        let notify = Arc::new(tokio::sync::Notify::new());
        let waker = std::task::Waker::from(Arc::new(al_net::TokioWaker::new(notify.clone())));

        loop {
            // Poll once per iteration. When the transports have data,
            // they fire the waker and the select below wakes to poll again.
            {
                let killed = {
                    self.driver
                        .lock()
                        .await
                        .poll(&mut std::task::Context::from_waker(&waker))
                };
                for tcp_id in killed {
                    let conn_id = match self.tcp_to_conn.get(&tcp_id) {
                        Ok(Some(&c)) => c,
                        Ok(None) => continue,
                        Err(e) => {
                            eprintln!("Network `tcp_to_conn get` failed: {e}");
                            continue;
                        }
                    };
                    if let Err(e) = self.conn_map.remove(&conn_id) {
                        eprintln!("Network `conn_map remove` failed: {e}");
                    }
                    if let Err(e) = self.tcp_to_conn.remove(&tcp_id) {
                        eprintln!("Network `tcp_to_conn remove` failed: {e}");
                    }
                    if let Err(e) = self.live.write().await.retain(|(_, &i)| i != conn_id) {
                        eprintln!("Network `live retain` failed: {e}");
                    }
                    if let Err(e) = self.events.send(NetEvent::Disconnected(conn_id)) {
                        eprintln!("Network `conn_map remove` failed: {e}");
                    }
                }
            }

            // Accept future (if server)
            let accept_fut = async {
                match &listener {
                    Some(l) => l.accept().await,
                    None => std::future::pending().await,
                }
            };

            tokio::select! {
                biased;
                _ = self.token.cancelled() => break,

                accepted = accept_fut => {
                    let (stream, _peer) = match accepted {
                        Ok(x) => x,
                        Err(e) => { eprintln!("Chat server `accept` error: {e}"); continue; }
                    };
                    let (tcp, socket) = match Tcp::<N>::new_responder(
                        al_secure::noise::handshake_pattern::HandshakePattern::NN,
                        Vec::new(), None, None
                    ) {
                        Ok(x) => x,
                        Err(e) => { eprintln!("Chat server `noise` error: {e}"); continue; }
                    };
                    if let Err(e) = self.add_tcp(tcp, socket, stream).await {
                        eprintln!("Chat server `add_tcp` failed: {e}");
                        continue;
                    }
                }

                item = self.net_in.recv() => {
                    let mut driver = self.driver.lock().await;

                    let mut batch = Vec::new();
                    if let Ok(item) = item { batch.push(item); }
                    while let Ok(Some(extra)) = self.net_in.try_recv() { batch.push(extra); }

                    for (conn_id, bytes) in batch {
                        if conn_id == ConnId::MAX {
                            let msg = match DynMessage::from_format_slice(&bytes) {
                                Ok(msg) => msg,
                                Err(e) => {
                                    eprintln!("Chat server `broadcast` failed: {e}");
                                    continue;
                                }
                            };
                            let mut broadcast = match msg.into_event().map(|(e, _)| e.downcast::<Broadcast>()) {
                                Some(Ok(bcst)) => bcst,
                                _ => {
                                    eprintln!("Chat server `broadcast downcast` failed.");
                                    continue;
                                }
                            };
                            let except = broadcast.except.take().unwrap_or_default();
                            let payload = broadcast.msg;

                            match self.live.read().await.values() {
                                Ok(values) => for live_id in values {
                                    if except.contains(&live_id) { continue; }
                                    if let Ok(Some(&tcp_id)) = self.conn_map.get(&live_id) {
                                        if let Err(e) = driver.deliver_to(tcp_id, payload.clone()) {
                                            eprintln!("Chat server `broadcast deliver_to` failed: {e}");
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Chat server `live values` failed: {e}");
                                    continue;
                                }
                            }
                        } else {
                            match self.conn_map.get(&conn_id) {
                                Ok(Some(&tcp_id)) => {
                                    if let Err(e) = driver.deliver_to(tcp_id, bytes) {
                                        eprintln!("deliver_to failed: {e}");
                                    }
                                }
                                Ok(None) => {},
                                Err(e) => eprintln!("conn_map lookup failed: {e}"),
                            }
                        }
                    }
                    drop(driver);
                }

                _ = notify.notified() => {}
            }
        }

        // Attempt to join gracefully
        if tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while self.socket_tasks.join_next().await.is_some() {}
        })
        .await
        .is_err()
        {
            self.socket_tasks.shutdown().await;
        }
    }
}
