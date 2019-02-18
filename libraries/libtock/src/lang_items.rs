use core::intrinsics;
use core::panic::PanicInfo;

// Panic handler. Adapted from `panic-abort` crate
#[no_mangle]
#[panic_handler]
pub unsafe extern "C" fn panic_fmt(_info: &PanicInfo) -> ! {
    intrinsics::abort();
}

#[lang = "start"]
extern "C" fn start<T>(main: fn() -> T, _argc: isize, _argv: *const *const u8) -> i32
where
    T: Termination,
{
    main().report()
}

pub trait Termination {
    fn report(self) -> i32;
}

impl Termination for () {
    fn report(self) -> i32 {
        0
    }
}
