use crate::{SliceDebug, Transport, TransportError, TransportItemRequirements};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

pub trait FilterFn<T>: Send + Sync + 'static {
    fn filter(&self, item: &T) -> bool;
}
impl<T, F> FilterFn<T> for F
where
    F: Fn(&T) -> bool + Send + Sync + 'static,
{
    fn filter(&self, item: &T) -> bool {
        self(item)
    }
}

/// Format for subscribing to a Publisher
pub enum SubscribeFormat<T: TransportItemRequirements> {
    All(Arc<dyn Transport<T>>),
    Channel(Arc<dyn Transport<T>>, String),
}

impl<T: TransportItemRequirements> From<Arc<dyn Transport<T>>> for SubscribeFormat<T> {
    fn from(transport: Arc<dyn Transport<T>>) -> Self {
        SubscribeFormat::All(transport)
    }
}

impl<T: TransportItemRequirements, R: Transport<T>> From<Arc<R>> for SubscribeFormat<T> {
    fn from(transport: Arc<R>) -> Self {
        SubscribeFormat::All(transport)
    }
}

impl<T: TransportItemRequirements, S: AsRef<str>> From<(Arc<dyn Transport<T>>, S)>
    for SubscribeFormat<T>
{
    fn from(value: (Arc<dyn Transport<T>>, S)) -> Self {
        SubscribeFormat::Channel(value.0, value.1.as_ref().to_string())
    }
}

impl<T: TransportItemRequirements, R: Transport<T>, S: AsRef<str>> From<(Arc<R>, S)>
    for SubscribeFormat<T>
{
    fn from(value: (Arc<R>, S)) -> Self {
        SubscribeFormat::Channel(value.0, value.1.as_ref().to_string())
    }
}

/* ********************
  Publisher
******************** */
pub struct Publisher<T> {
    subscribers: Mutex<Vec<Arc<dyn Transport<T>>>>,
    subscriber_channels: Mutex<HashMap<String, usize>>,
    filters: Mutex<Vec<Arc<dyn FilterFn<T>>>>,
    channels: Mutex<Vec<Arc<Mutex<Vec<Arc<dyn Transport<T>>>>>>>,
}

impl<T> std::fmt::Debug for Publisher<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.subscribers.lock() {
            Ok(guard) => f
                .debug_struct("Publisher")
                .field("subscribers_count", &guard.len())
                .field("subscribers", &SliceDebug::new(&*guard))
                .finish(),
            Err(e) => f
                .debug_struct("Publisher")
                .field("subscribers", &format!("<LockPoisoned>: {}", e.to_string()))
                .finish(),
        }
    }
}

impl<T: TransportItemRequirements> From<Publisher<T>> for Arc<dyn Transport<T>> {
    fn from(publisher: Publisher<T>) -> Self {
        Arc::new(publisher)
    }
}

impl<T: TransportItemRequirements> From<crate::List<T>> for Publisher<T> {
    fn from(list: crate::List<T>) -> Self {
        let publisher = Publisher::new();
        let _ = list.with(|transports| {
            for transport in transports.iter() {
                let _ = publisher.subscribe(transport.clone());
            }
        });
        publisher
    }
}

impl<T: TransportItemRequirements> Publisher<T> {
    pub fn new() -> Self {
        Self {
            subscribers: Mutex::new(Vec::new()),
            subscriber_channels: Mutex::new(HashMap::new()),
            filters: Mutex::new(Vec::new()),
            channels: Mutex::new(Vec::new()),
        }
    }

    pub fn subscribe(
        &self,
        transport: impl Into<SubscribeFormat<T>>,
    ) -> Result<(), TransportError> {
        match transport.into() {
            SubscribeFormat::All(transport) => match self.subscribers.lock() {
                Ok(mut guard) => {
                    guard.push(transport);
                    Ok(())
                }
                Err(e) => {
                    return Err(TransportError::Transport(format!(
                        "Error acquiring subscribers lock: {}",
                        e.to_string()
                    )))
                }
            },
            SubscribeFormat::Channel(transport, channel) => {
                let channel_index = match self.subscriber_channels.lock() {
                    Ok(guard) => guard.get(&channel).cloned().ok_or_else(|| {
                        TransportError::Transport(format!(
                            "Channel '{}' not found in publisher",
                            channel
                        ))
                    })?,
                    Err(e) => {
                        return Err(TransportError::Transport(format!(
                            "Error acquiring subscriber_channels lock: {}",
                            e.to_string()
                        )))
                    }
                };
                match self.channels.lock() {
                    Ok(guard) => match guard.get(channel_index) {
                        Some(channel_transports_mutex) => match channel_transports_mutex.lock() {
                            Ok(mut channel_transports) => channel_transports.push(transport),
                            Err(e) => {
                                return Err(TransportError::Transport(format!(
                                    "Error acquiring channel transports lock: {}",
                                    e.to_string()
                                )))
                            }
                        },
                        None => {
                            return Err(TransportError::Transport(format!(
                                "Channel index '{}' not found in publisher",
                                channel_index
                            )))
                        }
                    },
                    Err(e) => {
                        return Err(TransportError::Transport(format!(
                            "Error acquiring channels lock: {}",
                            e.to_string()
                        )))
                    }
                };
                Ok(())
            }
        }
    }

    pub fn add_channel(
        &self,
        name: impl AsRef<str>,
        filter: Arc<dyn FilterFn<T>>,
    ) -> Result<usize, TransportError> {
        let index;
        match self.channels.lock() {
            Ok(mut channels) => match self.filters.lock() {
                Ok(mut filters) => match self.subscriber_channels.lock() {
                    Ok(mut subscriber_channels) => {
                        channels.push(Arc::new(Mutex::new(Vec::new())));
                        filters.push(filter);
                        index = channels.len() - 1;
                        subscriber_channels.insert(name.as_ref().to_string(), index);
                    }
                    Err(e) => {
                        return Err(TransportError::Transport(format!(
                            "Error acquiring subscriber_channels lock: {}",
                            e.to_string()
                        )))
                    }
                },
                Err(e) => {
                    return Err(TransportError::Transport(format!(
                        "Error acquiring filters lock: {}",
                        e.to_string()
                    )))
                }
            },
            Err(e) => {
                return Err(TransportError::Transport(format!(
                    "Error acquiring channels lock: {}",
                    e.to_string()
                )))
            }
        }
        Ok(index)
    }
}

impl<T: TransportItemRequirements> Transport<T> for Publisher<T> {
    fn send_blocking(&self, data: T) -> Result<(), TransportError> {
        let mut err = vec![];
        // Send to all subscribers
        if let Ok(guard) = self.subscribers.lock() {
            for transport in guard.iter() {
                if let Err(e) = transport.send_blocking(data.clone()) {
                    err.push(e);
                }
            }
        }
        // Send to channel subscribers
        if let Ok(channels) = self.channels.lock() {
            if let Ok(filters) = self.filters.lock() {
                for (i, channel_mutex) in channels.iter().enumerate() {
                    if filters[i].filter(&data) {
                        if let Ok(channel_transports) = channel_mutex.lock() {
                            for transport in channel_transports.iter() {
                                if let Err(e) = transport.send_blocking(data.clone()) {
                                    err.push(e);
                                }
                            }
                        }
                    }
                }
            }
        }
        if !err.is_empty() {
            Err(TransportError::Transport(format!(
                "Error(s) occured when sending to subscribers: {:?}",
                err
            )))
        } else {
            Ok(())
        }
    }

    fn send_batch_blocking(&self, data: Vec<T>) -> Result<(), TransportError> {
        let mut err = vec![];
        // Send to all subscribers
        if let Ok(guard) = self.subscribers.lock() {
            for transport in guard.iter() {
                if let Err(e) = transport.send_batch_blocking(data.clone()) {
                    err.push(e);
                }
            }
        }
        // Send to channel subscribers
        if let Ok(channels) = self.channels.lock() {
            if let Ok(filters) = self.filters.lock() {
                for (i, channel_mutex) in channels.iter().enumerate() {
                    let data = data
                        .iter()
                        .filter(|d| filters[i].filter(d))
                        .cloned()
                        .collect::<Vec<T>>();
                    if !data.is_empty() {
                        if let Ok(channel_transports) = channel_mutex.lock() {
                            for transport in channel_transports.iter() {
                                if let Err(e) = transport.send_batch_blocking(data.clone()) {
                                    err.push(e);
                                }
                            }
                        }
                    }
                }
            }
        }
        if !err.is_empty() {
            Err(TransportError::Transport(format!(
                "Error(s) occured when sending to subscribers: {:?}",
                err
            )))
        } else {
            Ok(())
        }
    }

    fn recv_blocking(&self) -> Result<T, TransportError> {
        Err(TransportError::UnSupported(
            "Publisher transport does not support recv".to_string(),
        ))
    }

    fn recv_avaliable_blocking(&self) -> Result<Vec<T>, TransportError> {
        Err(TransportError::UnSupported(
            "Publisher transport does not support recv".to_string(),
        ))
    }

    fn try_recv_blocking(&self) -> Result<Option<T>, TransportError> {
        Err(TransportError::UnSupported(
            "Publisher transport does not support recv".to_string(),
        ))
    }

    fn send(
        &self,
        data: T,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<(), TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async move {
            let transports = {
                match self.subscribers.lock() {
                    Ok(guard) => guard.clone(),
                    Err(e) => {
                        return Err(TransportError::Transport(format!(
                            "Error acquiring subscribers lock: {}",
                            e.to_string()
                        )))
                    }
                }
            };
            let channels = {
                match self.channels.lock() {
                    Ok(guard) => guard.clone(),
                    Err(e) => {
                        return Err(TransportError::Transport(format!(
                            "Error acquiring channels lock: {}",
                            e.to_string()
                        )))
                    }
                }
            };

            let filters = {
                match self.filters.lock() {
                    Ok(guard) => guard.clone(),
                    Err(e) => {
                        return Err(TransportError::Transport(format!(
                            "Error acquiring filters lock: {}",
                            e.to_string()
                        )))
                    }
                }
            };

            let mut err = vec![];
            // Send to all subscribers
            for transport in transports.iter() {
                if let Err(e) = transport.send(data.clone()).await {
                    err.push(e);
                }
            }
            // Send to channel subscribers
            for (i, channel_mutex) in channels.iter().enumerate() {
                if filters[i].filter(&data) {
                    let channel_transports = {
                        match channel_mutex.lock() {
                            Ok(guard) => guard.clone(),
                            Err(e) => {
                                return Err(TransportError::Transport(format!(
                                    "Error acquiring channel transports lock: {}",
                                    e.to_string()
                                )))
                            }
                        }
                    };
                    for transport in channel_transports.iter() {
                        if let Err(e) = transport.send(data.clone()).await {
                            err.push(e);
                        }
                    }
                }
            }
            if !err.is_empty() {
                Err(TransportError::Transport(format!(
                    "Error(s) occured when sending to subscribers: {:?}",
                    err
                )))
            } else {
                Ok(())
            }
        })
    }

    fn send_batch(
        &self,
        data: Vec<T>,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<(), TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async move {
            let transports = {
                match self.subscribers.lock() {
                    Ok(guard) => guard.clone(),
                    Err(e) => {
                        return Err(TransportError::Transport(format!(
                            "Error acquiring subscribers lock: {}",
                            e.to_string()
                        )))
                    }
                }
            };
            let channels = {
                match self.channels.lock() {
                    Ok(guard) => guard.clone(),
                    Err(e) => {
                        return Err(TransportError::Transport(format!(
                            "Error acquiring channels lock: {}",
                            e.to_string()
                        )))
                    }
                }
            };

            let filters = {
                match self.filters.lock() {
                    Ok(guard) => guard.clone(),
                    Err(e) => {
                        return Err(TransportError::Transport(format!(
                            "Error acquiring filters lock: {}",
                            e.to_string()
                        )))
                    }
                }
            };

            let mut err = vec![];
            // Send to all subscribers
            for transport in transports.iter() {
                if let Err(e) = transport.send_batch(data.clone()).await {
                    err.push(e);
                }
            }
            // Send to channel subscribers
            for (i, channel_mutex) in channels.iter().enumerate() {
                let data = data
                    .iter()
                    .filter(|d| filters[i].filter(d))
                    .cloned()
                    .collect::<Vec<T>>();
                if !data.is_empty() {
                    let channel_transports = {
                        match channel_mutex.lock() {
                            Ok(guard) => guard.clone(),
                            Err(e) => {
                                return Err(TransportError::Transport(format!(
                                    "Error acquiring channel transports lock: {}",
                                    e.to_string()
                                )))
                            }
                        }
                    };
                    for transport in channel_transports.iter() {
                        if let Err(e) = transport.send_batch(data.clone()).await {
                            err.push(e);
                        }
                    }
                }
            }

            if !err.is_empty() {
                Err(TransportError::Transport(format!(
                    "Error(s) occured when sending to subscribers: {:?}",
                    err
                )))
            } else {
                Ok(())
            }
        })
    }

    fn recv(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<T, TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async { self.recv_blocking() })
    }

    fn recv_avaliable(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<Vec<T>, TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async { self.recv_avaliable_blocking() })
    }

    fn try_recv(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<Option<T>, TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async { self.try_recv_blocking() })
    }
}

#[cfg(test)]
mod tests {
    use crate::{Publisher, Queue, Transport};
    use std::sync::Arc;

    #[tokio::test]
    async fn debug() {
        let publisher = Publisher::<u8>::new();
        assert_eq!(
            format!("{:?}", publisher),
            "Publisher { subscribers_count: 0, subscribers: [] }"
        );
        let _ = publisher.subscribe(Arc::new(Publisher::<u8>::new()));
        assert_eq!(
            format!("{:?}", publisher),
            "Publisher { subscribers_count: 1, subscribers: [Publisher { subscribers_count: 0, subscribers: [] }] }"
        );
        let _ = publisher.subscribe(Arc::new(Publisher::<u8>::new()));
        let _ = publisher.subscribe(Arc::new(Publisher::<u8>::new()));
        assert_eq!(
            format!("{:?}", publisher),
            "Publisher { subscribers_count: 3, subscribers: [Publisher { subscribers_count: 0, subscribers: [] }, Publisher { subscribers_count: 0, subscribers: [] }, Publisher { subscribers_count: 0, subscribers: [] }] }"
        );
        let _ = publisher.subscribe(Arc::new(Publisher::<u8>::new()));
        assert_eq!(
            format!("{:?}", publisher),
            "Publisher { subscribers_count: 4, subscribers: [Publisher { subscribers_count: 0, subscribers: [] }, Publisher { subscribers_count: 0, subscribers: [] }, Publisher { subscribers_count: 0, subscribers: [] }, +1 more...] }"
        );
    }

    #[tokio::test]
    async fn send_recv() {
        let publisher = Publisher::<u8>::new();
        let mut subscribers = vec![];
        for _ in 0..3 {
            let subscriber = Arc::new(Queue::<u8>::new());
            let _ = publisher.subscribe(subscriber.clone());
            subscribers.push(subscriber);
        }

        // Send `Command` asynchronously
        publisher.send(1).await.unwrap();
        publisher.send_batch(vec![2, 3]).await.unwrap();
        // Recv `String` asynchronously
        assert_eq!(subscribers[0].recv().await.unwrap(), 1);
        // Try recv `String` asynchronously
        assert_eq!(subscribers[0].try_recv().await.unwrap().unwrap(), 2);
        // Recv avaliable `String` asynchronously
        assert_eq!(subscribers[0].recv_avaliable().await.unwrap(), vec![3]);

        // Recv `String` asynchronously
        assert_eq!(subscribers[1].recv().await.unwrap(), 1);
        // Try recv `String` asynchronously
        assert_eq!(subscribers[1].try_recv().await.unwrap().unwrap(), 2);
        // Recv avaliable `String` asynchronously
        assert_eq!(subscribers[1].recv_avaliable().await.unwrap(), vec![3]);

        // Recv `String` asynchronously
        assert_eq!(subscribers[2].recv().await.unwrap(), 1);
        // Try recv `String` asynchronously
        assert_eq!(subscribers[2].try_recv().await.unwrap().unwrap(), 2);
        // Recv avaliable `String` asynchronously
        assert_eq!(subscribers[2].recv_avaliable().await.unwrap(), vec![3]);

        // Send `Command` synchronously
        publisher.send_blocking(1).unwrap();
        publisher.send_batch_blocking(vec![2, 3]).unwrap();
        tokio::time::sleep(std::time::Duration::from_nanos(1)).await;
        // Recv `String` synchronously
        assert_eq!(subscribers[0].recv_blocking().unwrap(), 1);
        // Try recv `String` synchronously
        assert_eq!(subscribers[0].try_recv_blocking().unwrap().unwrap(), 2);
        // Recv avaliable `String` synchronously
        assert_eq!(subscribers[0].recv_avaliable_blocking().unwrap(), vec![3]);

        // Recv `String` synchronously
        assert_eq!(subscribers[1].recv_blocking().unwrap(), 1);
        // Try recv `String` synchronously
        assert_eq!(subscribers[1].try_recv_blocking().unwrap().unwrap(), 2);
        // Recv avaliable `String` synchronously
        assert_eq!(subscribers[1].recv_avaliable_blocking().unwrap(), vec![3]);

        // Recv `String` synchronously
        assert_eq!(subscribers[2].recv_blocking().unwrap(), 1);
        // Try recv `String` synchronously
        assert_eq!(subscribers[2].try_recv_blocking().unwrap().unwrap(), 2);
        // Recv avaliable `String` synchronously
        assert_eq!(subscribers[2].recv_avaliable_blocking().unwrap(), vec![3]);
    }

    #[tokio::test]
    async fn threaded() {
        let publisher = Publisher::<u8>::new();
        let queue = Arc::new(Queue::<u8>::new());
        let _ = publisher.subscribe(queue.clone());
        let handle = std::thread::spawn(move || {
            let received = queue.recv_blocking().unwrap();
            assert_eq!(received, 42);
        });

        // Wait to ensure the other thread is receiving the data
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        publisher.send_blocking(42).unwrap();

        handle.join().unwrap();

        let tokio_publisher = Publisher::<u8>::new();
        let tokio_queue = Arc::new(Queue::<u8>::new());
        let _ = tokio_publisher.subscribe(tokio_queue.clone());
        let tokio_handle = tokio::spawn(async move {
            let received = tokio_queue.recv().await.unwrap();
            assert_eq!(received, 42);
        });

        // Wait to ensure the other thread is receiving the data
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        tokio_publisher.send_blocking(42).unwrap();

        tokio_handle.await.unwrap();
    }

    #[tokio::test]
    async fn filtered() {
        let publisher = Publisher::<u8>::new();
        let mut subscribers = vec![];
        let mut channels = vec![];
        for i in 0..3 {
            let channel_name = format!("channel_{}", i);
            let filter = Arc::new(move |item: &u8| *item % 3 == i as u8);
            let _ = publisher.add_channel(channel_name.clone(), filter);
            let subscriber = Arc::new(Queue::<u8>::new());
            let _ = publisher.subscribe((subscriber.clone(), &channel_name));
            subscribers.push(subscriber);
            channels.push(channel_name);
        }

        for item in 0..9u8 {
            publisher.send(item).await.unwrap();
        }

        for i in 0..3 {
            let expected: Vec<u8> = (0..9).filter(|x| *x % 3 == i as u8).collect();
            let received = subscribers[i].recv_avaliable().await.unwrap();
            assert_eq!(received, expected);
            assert_eq!(format!("channel_{}", i), channels[i]);
        }
    }
}
