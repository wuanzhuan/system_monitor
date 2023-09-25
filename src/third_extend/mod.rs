pub mod strings;

pub mod bytemuck {
    use bytemuck::*;
    pub fn cast_slice_truncate<A: NoUninit, B: AnyBitPattern>(a: &[A]) -> &[B] {
        let new_len = core::mem::size_of_val(a) / core::mem::size_of::<B>();
        unsafe { core::slice::from_raw_parts(a.as_ptr() as *const B, new_len) }
    } 
}
