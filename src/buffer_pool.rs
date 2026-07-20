//! Buffer pool for reducing allocations during mesh updates.

use std::sync::Mutex;

/// A pool of reusable Vec<f32> buffers for mesh vertex data.
///
/// This pool reduces per-frame allocations by reusing buffers across updates.
pub struct BufferPool {
    buffers: Mutex<Vec<Vec<f32>>>,
}

impl BufferPool {
    /// Creates a new empty buffer pool.
    pub const fn new() -> Self {
        Self {
            buffers: Mutex::new(Vec::new()),
        }
    }

    /// Takes a buffer from the pool, or allocates a new one if the pool is empty.
    pub fn take(&self, capacity_hint: usize) -> Vec<f32> {
        let mut buffers = self.buffers.lock().unwrap();
        buffers
            .pop()
            .map(|mut buf| {
                buf.clear();
                if buf.capacity() < capacity_hint {
                    buf.reserve(capacity_hint - buf.capacity());
                }
                buf
            })
            .unwrap_or_else(|| Vec::with_capacity(capacity_hint))
    }

    /// Returns a buffer to the pool for reuse.
    pub fn return_buf(&self, mut buffer: Vec<f32>) {
        buffer.clear();
        // Don't pool excessively large buffers
        if buffer.capacity() < 1024 * 1024 {
            let mut buffers = self.buffers.lock().unwrap();
            buffers.push(buffer);
        }
    }
}

impl Default for BufferPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Global buffer pool for mesh operations.
static MESH_BUFFER_POOL: BufferPool = BufferPool::new();

/// Takes a buffer from the global mesh buffer pool.
pub fn take_mesh_buffer(capacity_hint: usize) -> Vec<f32> {
    MESH_BUFFER_POOL.take(capacity_hint)
}

/// Returns a buffer to the global mesh buffer pool.
pub fn return_mesh_buffer(buffer: Vec<f32>) {
    MESH_BUFFER_POOL.return_buf(buffer);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_reuses_buffers() {
        let pool = BufferPool::new();
        let buf1 = pool.take(100);
        let ptr1 = buf1.as_ptr();
        pool.return_buf(buf1);
        let buf2 = pool.take(100);
        let ptr2 = buf2.as_ptr();
        // Same pointer means buffer was reused
        assert_eq!(ptr1, ptr2);
    }

    #[test]
    fn pool_clears_returned_buffers() {
        let pool = BufferPool::new();
        let mut buf = pool.take(10);
        buf.extend_from_slice(&[1.0, 2.0, 3.0]);
        pool.return_buf(buf);
        let buf2 = pool.take(10);
        assert_eq!(buf2.len(), 0);
    }

    #[test]
    fn pool_does_not_keep_huge_buffers() {
        let pool = BufferPool::new();
        let buf = vec![0.0; 2_000_000];
        pool.return_buf(buf);
        let buffers = pool.buffers.lock().unwrap();
        assert_eq!(buffers.len(), 0);
    }
}
