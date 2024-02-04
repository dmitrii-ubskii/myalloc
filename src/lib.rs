use std::{
	alloc::{GlobalAlloc, Layout},
	arch::asm,
	ptr::null_mut,
	sync::atomic::{AtomicUsize, Ordering},
};

pub struct Alloc {
	total: AtomicUsize,
}

impl Alloc {
	pub const fn new() -> Self {
		Self { total: AtomicUsize::new(0) }
	}

	pub fn check(&self) -> usize {
		self.total.load(Ordering::Relaxed)
	}
}

unsafe impl GlobalAlloc for Alloc {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		let size = layout.size();

		if size == 0 {
			return null_mut();
		}

		self.total.fetch_add(size, Ordering::AcqRel);
		let ptr;
		asm! {
			"mov rdi, 0",       // addr hint: NULL (let kernel choose)
			"mov rsi, {size}",  // size: mapping size
			"mov rdx, 3",       // PROT_READ | PROT_WRITE
			"mov r10, 0x22",    // MAP_ANONYMOUS | MAP_PRIVATE
			"mov r8, -1",       // fd: -1, since anonymous
			"mov r9, 0",        // offset: 0, since anonymous
			"mov rax, 9",       // mmap syscall number
			"syscall",
			"mov {ptr}, rax",   // return value
			size = in(reg) size,
			ptr = out(reg) ptr,
		};
		ptr
	}

	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		let size = layout.size();

		if ptr.is_null() {
			assert_eq!(size, 0);
			return;
		}

		self.total.fetch_sub(size, Ordering::AcqRel);
		asm! {
			"mov rdi, {ptr}",  // addr of the mmap to unmap
			"mov rsi, {size}", // size
			"mov rax, 11",     // munmap syscall number
			"syscall",
			ptr = in(reg) ptr,
			size = in(reg) size,
		};
	}
}

#[cfg(test)]
mod tests {
	use std::{thread::sleep, time::Duration};

	use super::*;

	#[global_allocator]
	static ALLOC: Alloc = Alloc::new();

	#[test]
	fn watch_process_ram() {
		{
			let _vec = vec![0u8; 0x4000_0000]; // 1 GiB
			sleep(Duration::from_secs(10)); // see resident memory jump up ...
		}
		sleep(Duration::from_secs(10)); // ... and back down
	}
}
