use core::future::Future;
use core::task::{Poll, Context};

pub fn block_on<F: Future>(f: F) -> F::Output {
    pin_mut!(f);

    loop {
        let waker = futures::task::noop_waker();
        let mut cx = Context::from_waker(&waker);
        loop {
            if let Poll::Ready(t) = f.as_mut().poll(&mut cx) {
                return t;
            } else {
                picorv32_rt::wfi();
            }
        }
    }
}