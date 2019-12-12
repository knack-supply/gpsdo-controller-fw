use embedded_hal::digital::v2::OutputPin;
use embedded_hal::spi::FullDuplex;
use async_trait::async_trait;
use alloc::boxed::Box;

pub struct OutputPinHoldGuard<'a, P: OutputPin> {
    out: &'a mut P
}

impl <'a, P: OutputPin> Drop for OutputPinHoldGuard<'a, P> {
    fn drop(&mut self) {
        self.out.set_high().ok().unwrap();
    }
}

pub trait OutputPinHold<P: OutputPin> {
    fn hold_low(&mut self) -> core::result::Result<OutputPinHoldGuard<P>, P::Error>;
}

impl <P: OutputPin> OutputPinHold<P> for P {
    fn hold_low(&mut self) -> core::result::Result<OutputPinHoldGuard<P>, P::Error> {
        self.set_low()?;
        Ok(OutputPinHoldGuard {
            out: self
        })
    }
}

#[macro_export]
macro_rules! nb_future {
    ($e:expr) => {
        ::futures::future::poll_fn(|_cx| {
            match $e {
                Err(nb::Error::Other(e)) => ::core::task::Poll::Ready(::core::result::Result::Err(e)),
                Err(nb::Error::WouldBlock) => ::core::task::Poll::Pending,
                Ok(x) => ::core::task::Poll::Ready(::core::result::Result::Ok(x)),
            }
        })
    }
}

#[async_trait(?Send)]
pub trait FullDuplexTransfer<Word, S: FullDuplex<Word>> {
    async fn transfer<'a>(&'a mut self, buf: &'a mut [Word]) -> core::result::Result<(), S::Error>;
}

#[async_trait(?Send)]
impl <Word: Copy, S: FullDuplex<Word>> FullDuplexTransfer<Word, S> for S {
    async fn transfer<'a>(&'a mut self, buf: &'a mut [Word]) -> core::result::Result<(), S::Error> {
        for b in buf.iter_mut() {
            let byte_to_send = *b;
            nb_future!(self.send(byte_to_send)).await?;
            *b = nb_future!(self.read()).await?;
        }
        Ok(())
    }
}
