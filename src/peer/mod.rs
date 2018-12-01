mod bytes_stream;


use futures::sync::mpsc;
use tokio::prelude::*;
use bytes::Bytes;
use error::Error;
use shared::Shared;
pub use self::bytes_stream::BytesStream;


type Receiver = mpsc::UnboundedReceiver<Bytes>;
type Sender = mpsc::UnboundedSender<Bytes>;


/// Represents an incoming connection
pub struct Peer {
    id: u64,
    bytes_stream: BytesStream,
    sender: Sender,
    receiver: Receiver,
    shared: Shared,
}

impl Peer {
    pub fn new(id: u64, bytes_stream: BytesStream, shared: Shared) -> Self {
        let (sender, receiver) = mpsc::unbounded();

        {
            let mut peers = shared.peers.write();
            peers.insert(id);
        }

        Self {
            id,
            bytes_stream,
            sender,
            receiver,
            shared,
        }
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        let mut peers = self.shared.peers.write();
        peers.remove(&self.id);
    }
}

impl Future for Peer {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        // FIXME: potential starvation of socket stream?
        while let Async::Ready(Some(val)) = self.receiver.poll().unwrap() {
            self.bytes_stream.fill_write_buffer(&val);
        }

        let _ = self.bytes_stream.poll_flush()?;

        match try_ready!(self.bytes_stream.poll()) {
            Some(data) => {
                debug!("Received {} bytes", data.len());
            },
            None => {
                debug!("Closing connection: {}", self.id);
                return Ok(Async::Ready(()));
            },
        }

        Ok(Async::NotReady)
    }
}
