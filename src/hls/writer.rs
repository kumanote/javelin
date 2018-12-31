use log::{debug, error, warn};
use futures::try_ready;
use tokio::prelude::*;
use super::{
    transport_stream::Buffer as TsBuffer,
};
use crate::media::{self, Media, avc};


pub struct Writer {
    receiver: media::Receiver,
    write_interval: u64,
    next_write: u64,
    keyframe_counter: usize,
    buffer: TsBuffer,
    shared_state: avc::SharedState,
}

impl Writer {
    pub fn new(receiver: media::Receiver) -> Self {
        let write_interval = 2000; // milliseconds
        let next_write = write_interval; // milliseconds

        Self {
            receiver,
            write_interval,
            next_write,
            keyframe_counter: 0,
            buffer: TsBuffer::new(),
            shared_state: avc::SharedState::new(),
        }
    }
}


impl Future for Writer {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Some(media) = try_ready!(self.receiver.poll()) {
            match media {
                Media::H264(timestamp, bytes) => {
                    let timestamp = u64::from(timestamp.value);

                    let packet = match avc::Packet::try_from_buf(bytes, timestamp, &self.shared_state) {
                        Err(why) => {
                            error!("Failed to build packet: {:?}", why);
                            continue;
                        },
                        Ok(p) => p
                    };

                    if packet.is_sequence_header() {
                        debug!("Received sequence header");
                        continue;
                    }

                    if packet.is_keyframe() {
                        if timestamp >= self.next_write {
                            // TODO: Use publishing application name as output directory and check if exists.
                            let filename = format!("{}-{}-{}.ts", "test", timestamp, self.keyframe_counter);
                            let path = format!("./tmp/stream/{}", filename);
                            self.buffer.write_to_file(&path).unwrap();
                            self.next_write += self.write_interval;
                        }

                        self.keyframe_counter += 1;
                    }

                    if let Err(why) = self.buffer.push_video(&packet) {
                        warn!("Failed to put data into buffer: {:?}", why);
                    }
                },
                Media::AAC(_, _) => (),
            }
        }

        Ok(Async::Ready(()))
    }
}
