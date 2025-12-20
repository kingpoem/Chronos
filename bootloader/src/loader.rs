pub fn load_kernel(hartid: usize, dtb: usize) -> ! {
    // 内核入口地址（Bootloader 0x80200000 + 128KB）
    const KERNEL_ENTRY: usize = 0x80220000;

    // 跳转到内核
    unsafe {
        let kernel_entry: extern "C" fn(usize, usize) -> ! = core::mem::transmute(KERNEL_ENTRY);
        kernel_entry(hartid, dtb);
    }
}
