use std::io;
use std::any::Any;
use std::io::{IoSlice, Read};

use anyhow::Error;
use axum::body::Bytes;

use futures::{Stream, StreamExt};

use tokio::sync::mpsc::{
    Sender,
};


use wasi_common::{WasiFile, file::FileType};

pub struct StdinStream<S> {
    body: S,
}

impl<S> StdinStream<S> {
    pub fn new(body: S) -> Self {
        Self { body }
    }
}

#[wiggle::async_trait]
impl<S> WasiFile for StdinStream<S>
    where
        S: 'static,
        S: Sync + Send + Unpin,
        S: Stream<Item=Result<Bytes, hyper::Error>>,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn get_filetype(&mut self) -> Result<FileType, Error> {
        Ok(FileType::BlockDevice)
    }

    async fn read_vectored<'a>(&mut self, bufs: &mut [io::IoSliceMut<'a>]) -> Result<u64, Error> {
        let payload = match self.body.next().await {
            None => return Ok(0),
            Some(payload) => payload,
        }?;

        let nread = payload.as_ref().read_vectored(bufs)?;

        Ok(nread.try_into()?)
    }

    async fn readable(&self) -> Result<(), Error> {
        Ok(())
    }
}

pub struct StdoutChan {
    tx: Sender<Result<Vec<u8>, io::Error>>,
}

impl StdoutChan {
    pub fn new(tx: Sender<Result<Vec<u8>, io::Error>>) -> Self {
        Self { tx }
    }
}

#[wiggle::async_trait]
impl WasiFile for StdoutChan {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn get_filetype(&mut self) -> Result<FileType, Error> {
        Ok(FileType::BlockDevice)
    }

    async fn write_vectored<'a>(&mut self, bufs: &[IoSlice<'a>]) -> Result<u64, Error> {
        let mut nwritten = 0_usize;

        for buf in bufs {
            let v = buf.to_vec();

            nwritten += v.len();

            self.tx.send(Ok(v)).await?;
        }

        Ok(nwritten.try_into()?)
    }

    async fn writable(&self) -> Result<(), Error> {
        Ok(())
    }
}

