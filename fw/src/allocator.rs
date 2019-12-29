#![cfg(not(test))]
use alloc::alloc::{Alloc, Layout, AllocErr, handle_alloc_error};

use linked_list_allocator::Heap;
use picorv32::interrupt;
use picorv32::interrupt::Mutex;
use core::ptr::NonNull;
use core::alloc::GlobalAlloc;

pub struct RISCVHeap {
    heap: Mutex<Heap>,
}

impl RISCVHeap {

    /// Create a new UNINITIALIZED heap allocator
    ///
    /// You must initialize this heap using the
    /// [`init`](struct.RISCVHeap.html#method.init) method before using the allocator.
    pub const fn empty() -> Self {
        Self {
            heap: Mutex::new(Heap::empty()),
        }
    }

    /// Initializes the heap
    ///
    /// This function must be called BEFORE you run any code that makes use of the
    /// allocator.
    ///
    /// `start_addr` is the address where the heap will be located.
    ///
    /// `size` is the size of the heap in bytes.
    ///
    /// Note that:
    ///
    /// - The heap grows "upwards", towards larger addresses. Thus `end_addr` must
    ///   be larger than `start_addr`
    ///
    /// - The size of the heap is `(end_addr as usize) - (start_addr as usize)`. The
    ///   allocator won't use the byte at `end_addr`.
    ///
    /// # Unsafety
    ///
    /// Obey these or Bad Stuff will happen.
    ///
    /// - This function must be called exactly ONCE.
    /// - `size > 0`
    pub unsafe fn init(&self, start_addr: usize, size: usize) {
        interrupt::free(|cs| {
            let heap = (self.heap.borrow(cs) as *const Heap as *mut Heap).as_mut().unwrap();
            heap.init(start_addr, size);
        });
    }
}

unsafe impl<'a> Alloc for &'a mut RISCVHeap {
    unsafe fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
        interrupt::free(|cs| {
            let heap = (self.heap.borrow(cs) as *const Heap as *mut Heap).as_mut().unwrap();
            heap.allocate_first_fit(layout)
        })
    }

    unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        interrupt::free(|cs| {
            let heap = (self.heap.borrow(cs) as *const Heap as *mut Heap).as_mut().unwrap();
            heap.deallocate(ptr, layout);
        });
    }
}

unsafe impl GlobalAlloc for RISCVHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match interrupt::free(|cs| {
            let heap = (self.heap.borrow(cs) as *const Heap as *mut Heap).as_mut().unwrap();
            heap.allocate_first_fit(layout)
        }) {
            Ok(mut mem) => mem.as_mut() as *mut u8,
            Err(_e) => core::ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        match NonNull::new(ptr) {
            Some(ptr) => {
                interrupt::free(|cs| {
                    let heap = (self.heap.borrow(cs) as *const Heap as *mut Heap).as_mut().unwrap();
                    heap.deallocate(ptr, layout);
                });
            },
            None => handle_alloc_error(layout),
        };
    }
}