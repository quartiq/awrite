//! ```
//! # tokio_test::block_on(async {
//! use embedded_io::Write;
//! use embedded_io_async::Write as Awrite;
//! use awrite::{awrite, awriteln, AwriteBuf};
//!
//! let mut async_sink = Vec::<u8>::new();
//! let mut buf = AwriteBuf::new([0u8; 32], &mut async_sink);
//!
//! awrite!(buf, "Hello").unwrap();
//! awriteln!(buf, "{:?} {}", 7, "bar").unwrap();
//! awriteln!(&mut buf).unwrap();
//!
//! assert_eq!(awriteln!(buf, "{:032}", 0),
//!     Err(embedded_io::WriteFmtError::Other(
//!         awrite::Error::Sync(embedded_io::SliceWriteError::Full))
//! ));
//!
//! assert_eq!(core::str::from_utf8(&async_sink).unwrap(), "Hello7 bar\n\n");
//!
//! let mut async_sink = [0u8; 8];
//! let mut slic = &mut async_sink[..];
//! let mut buf = AwriteBuf::new([0u8; 16], &mut slic);
//!
//! awriteln!(buf, "{:07}", 0).unwrap();
//!
//! assert_eq!(awriteln!(buf, "{:08}", 0),
//!     Err(embedded_io::WriteFmtError::Other(
//!         awrite::Error::Async(embedded_io::SliceWriteError::Full))
//! ));
//! # })
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(async_fn_in_trait)]
use core::fmt::Debug;
use embedded_io::ErrorType;

/// ```
/// use awrite::AwriteBuf;
///
/// // Async target (`embedded_io_async::Write`)
/// let mut target = Vec::<u8>::new();
///
/// // Borrowed scratch (`AsRef<[u8]> + `AsMut<[u8]>`)
/// let mut scratch = [0u8; 32];
/// let _ = AwriteBuf::new(&mut scratch[..], &mut target);
///
/// // Owned scratch
/// let _ = AwriteBuf::new(scratch, target);
/// ```
#[derive(Debug, Default, Clone, PartialEq, PartialOrd)]
pub struct AwriteBuf<T, U> {
    // Could also go for embedded_io::Write + AsRef<[u8]> + Seek instead of pos...
    buf: T,
    sink: U,
    pos: usize,
}

impl<T, U> AwriteBuf<T, U> {
    pub fn new(buf: T, sink: U) -> Self {
        Self { buf, sink, pos: 0 }
    }

    pub fn into_sink(self) -> U {
        self.sink
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Error<E> {
    Sync(embedded_io::SliceWriteError),
    Async(E),
}

impl<E: embedded_io::Error> embedded_io::Error for Error<E> {
    fn kind(&self) -> embedded_io::ErrorKind {
        match self {
            Self::Async(e) => e.kind(),
            Self::Sync(e) => e.kind(),
        }
    }
}

impl<T, U: ErrorType> ErrorType for AwriteBuf<T, U> {
    type Error = Error<U::Error>;
}

// Sync Write behavior like &mut [u8]
impl<T: AsMut<[u8]>, U: ErrorType> embedded_io::Write for AwriteBuf<T, U> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let mut sli = &mut self.buf.as_mut()[self.pos..];
        let written = sli.write(buf).map_err(Error::Sync)?;
        self.pos += written;
        Ok(written)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>, U: embedded_io_async::Write> embedded_io_async::Write
    for AwriteBuf<T, U>
{
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        embedded_io::Write::write(self, buf)
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.sink
            .write_all(&self.buf.as_ref()[..self.pos])
            .await
            .map_err(Error::Async)?;
        self.pos = 0;
        Ok(())
    }
}

#[macro_export]
macro_rules! awrite {
    ($aw:expr, $($tt:tt)*) => {
        match write!($aw, $($tt)*) {
            Ok(_) => embedded_io_async::Write::flush(&mut $aw).await.map_err(Into::into),
            e => e
        }
    };
}

#[macro_export]
macro_rules! awriteln {
    ($aw:expr $(,)?) => {
        awrite!($aw, "\n")
    };
    ($aw:expr, $($tt:tt)*) => {
        match writeln!($aw, $($tt)*) {
            Ok(_) => embedded_io_async::Write::flush(&mut $aw).await.map_err(Into::into),
            e => e
        }
    };
}
