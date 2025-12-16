// Lockless ring buffer for high-throughput event streaming

use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, Ordering};

#[repr(align(64))]
pub struct AlignedUsize(pub AtomicUsize);

pub struct RingBuffer<T> {
    buffer: Vec<MaybeUninit<T>>,
    capacity: usize,
    mask: usize,  //This does wraparound for free using fast bitmasking, instead of slow modulo.
    write_idx: AlignedUsize,
    read_idx: AlignedUsize,
}

unsafe impl<T: Send> Send for RingBuffer<T> {}
unsafe impl<T: Send> Sync for RingBuffer<T> {}

impl<T> RingBuffer<T> {

    pub fn new(capacity: usize) -> Self {
        assert!(capacity.is_power_of_two(), "Capacity must be power of 2");
        assert!(capacity > 1, "Capacity must be > 1");
        
        // Pre-allocate buffer with uninitialized memory
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(MaybeUninit::uninit());  //They are uninitialized (MaybeUninit).
        }

        Self {
            buffer,
            capacity,
            mask: capacity - 1,
            write_idx: AlignedUsize(AtomicUsize::new(0)),
            read_idx: AlignedUsize(AtomicUsize::new(0)),
        }
    }
  
    pub fn push(&self,item:T)->bool{
        let write = self.write_idx.0.load(Ordering::Relaxed);
        let read = self.read_idx.0.load(Ordering::Acquire);  //why accquire here (Acquiew ensure Producer sees all consumer-side memory writes before the consumer updated read_idx.)
        //Consumer updates read_idx with Release
        //Producer must see that update correctly
        let next_write = (write+1) & self.mask;

        if next_write == read {  //Cannot overwrite unread data.
            return false;
        }

        unsafe {  //happens before publishng write index
            let slot = self.buffer.get_unchecked(write);
            let ptr = slot.as_ptr() as *mut T;
            ptr.write(item);
        }
        self.write_idx.0.store(next_write,Ordering::Release);  //Publish the new write index (Release)
        true
    }

    pub fn push_spin(&mut self,item : T,max_spins: usize)->bool
    where
       T: Clone,
    {
        for _ in 0..max_spins{
            if self.push(item.clone()){
                return true;
            }
            // Hint to CPU: we're spinning (reduces power, helps hyperthreading)
            std::hint::spin_loop();
        }
        false  //still full after spining
    }
    pub fn try_pop(&self)->Option<T>{
        //Load current read index (relaxed)
        let read = self.read_idx.0.load(Ordering::Relaxed);  //Consumer owns read_idx. No ordering required for loading.
        let write = self.write_idx.0.load(Ordering::Acquire);  //Producer stored write_idx using Release.
        //Consumer must see all the writes that happened before producer’s Release.Acquire guarantees:Consumer sees the correct data written into slot before reading it.

        if read == write {
            return None; //No unread data.
        }

        let item = unsafe {

                let slot = self.buffer.get_unchecked(read);
                let ptr = slot.as_ptr() as *mut T;
                ptr.read()
        };
        let next_read = (read+1) &self.mask;
        self.read_idx.0.store(next_read,Ordering::Release); //Consumer uses Release because:
        //Consumer is saying: "I finished reading this slot, it is now free."
        //Producer loads read_idx with Acquire in push.
        Some(item)
        
    }  
    pub fn pop_spin(&self,max_spins:usize)->Option<T>{
        for i in 0..max_spins {
            if let Some(item) = self.try_pop(){
                return Some(item);
            }
            // first 100 iteration : tight spin (low latency)
            //after 100 yield to schedular (save cpu)
            if i > 100 {
                std::thread::yield_now();
            }else{
                std::hint::spin_loop();
            }
        }
        None
    }
    //Drain multiple item at once (batched read)
    //more efficent then calling try_pop() in loop (becasue we only update read_idx once)

    pub fn drain_batch(&self,max_items:usize)->Vec<T>{
        let mut batch = Vec::with_capacity(max_items);

        let read = self.read_idx.0.load(Ordering::Relaxed);
        let write = self.write_idx.0.load(Ordering::Acquire);

        let available = if write>= read{
            write - read
        }else {
            self.capacity - read + write
        };
        let to_read = available.min(max_items);

        let mut current = read;
        
        for _ in 0..to_read {
            let item = unsafe {
                let slot = self.buffer.get_unchecked(current);
                let ptr = slot.as_ptr();
                ptr.read()
            };
            batch.push(item);
            current = (current+1) & self.mask;
        }
        if !batch.is_empty() {
            self.read_idx.0.store(current,Ordering::Release);
        }
        batch
        
    }
}



