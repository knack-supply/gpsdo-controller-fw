use shared_bus;

use bare_metal::Mutex;

pub struct SharedBusMutex<T>(Mutex<T>);

impl<T> shared_bus::BusMutex<T> for SharedBusMutex<T> {
    fn create(v: T) -> SharedBusMutex<T> {
        SharedBusMutex(Mutex::new(v))
    }

    fn lock<R, F: FnOnce(&T) -> R>(&self, f: F) -> R {
        picorv32::interrupt::free(|cs| {
            let v = self.0.borrow(cs);
            f(v)
        })
    }
}

pub type SharedBusManager<L, P> = shared_bus::BusManager<SharedBusMutex<L>, P>;
