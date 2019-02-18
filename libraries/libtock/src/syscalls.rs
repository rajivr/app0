use crate::result::Result;

// Some drivers might pass error via a callback in `arg0`. If the driver wants
// to be cheeky, it can also use `arg1` or `arg2`. So even though its `usize` at
// type level, but in reality it would be carrying a negative `isize` value. In
// such a scenario use `result::UsizeError`
//
// We get around a simliar issue in `subscribe`, `command`, `allow` and `memop`
// by doing implicit type conversion from usize to size in the `asm!` block.
pub(crate) struct CallbackMessage {
    arg0: usize,
    arg1: usize,
    arg2: usize,
    userdata: usize,
}

#[allow(dead_code)]
impl CallbackMessage {
    pub fn new(arg0: usize, arg1: usize, arg2: usize, userdata: usize) -> CallbackMessage {
        CallbackMessage {
            arg0: arg0,
            arg1: arg1,
            arg2: arg2,
            userdata: userdata,
        }
    }

    pub fn get_arg0(&self) -> usize {
        self.arg0
    }

    pub fn get_arg1(&self) -> usize {
        self.arg1
    }

    pub fn get_arg2(&self) -> usize {
        self.arg2
    }

    pub fn get_userdata(&self) -> usize {
        self.userdata
    }
}

pub fn yieldk() {
    // Note: A process stops yielding when there is a callback ready to run,
    // which the kernel executes by modifying the stack frame pushed by the
    // hardware. The kernel copies the PC value from the stack frame to the LR
    // field, and sets the PC value to callback to run. When this frame is
    // unstacked during the interrupt return, the effectively clobbers the LR
    // register.
    //
    // At this point, the callback function is now executing, which may itself
    // clobber any of the other caller-saved registers. Thus we mark this
    // inline assembly as conservatively clobbering all caller-saved registers,
    // forcing yield to save any live registers.
    //
    // Upon direct observation of this function, the LR is the only register
    // that is live across the SVC invocation, however, if the yield call is
    // inlined, it is possible that the LR won't be live at all (commonly seen
    // for the `loop { yieldk(); }` idiom) or that other registers are live,
    // thus it is important to let the compiler do the work here.
    //
    // According to the AAPCS: A subroutine must preserve the contents of the
    // registers r4-r8, r10, r11 and SP (and r9 in PCS variants that designate
    // r9 as v6) As our compilation flags mark r9 as the PIC base register, it
    // does not need to be saved. Thus we must clobber r0-3, r12, and LR
    unsafe {
        asm!(
            "svc 0"
            :
            :
            : "memory", "r0", "r1", "r2", "r3", "r12", "lr"
            : "volatile");
    }
}

pub(crate) unsafe fn subscribe(
    major: usize,
    minor: usize,
    callback: *const unsafe extern "C" fn(usize, usize, usize, usize),
    userdata: usize,
) -> Result<usize> {
    let res: isize;

    asm!("svc 1" : "={r0}"(res)
         : "{r0}"(major) "{r1}"(minor) "{r2}"(callback) "{r3}"(userdata)
         : "memory"
         : "volatile");

    if res < 0 {
        Err(res.into())
    } else {
        Ok(res as usize)
    }
}

pub(crate) unsafe fn command(
    major: usize,
    minor: usize,
    arg1: usize,
    arg2: usize,
) -> Result<usize> {
    let res: isize;

    asm!("svc 2" : "={r0}"(res)
         : "{r0}"(major) "{r1}"(minor) "{r2}"(arg1) "{r3}"(arg2)
         : "memory"
         : "volatile");

    if res < 0 {
        Err(res.into())
    } else {
        Ok(res as usize)
    }
}

pub(crate) unsafe fn allow(major: usize, minor: usize, ptr: *mut u8, len: usize) -> Result<usize> {
    let res: isize;

    asm!("svc 3" : "={r0}"(res)
         : "{r0}"(major) "{r1}"(minor) "{r2}"(ptr) "{r3}"(len)
         : "memory"
         : "volatile");

    if res < 0 {
        Err(res.into())
    } else {
        Ok(res as usize)
    }
}

pub(crate) unsafe fn memop(major: u32, arg1: usize) -> Result<usize> {
    let res: isize;

    asm!("svc 4" : "={r0}"(res)
                 : "{r0}"(major) "{r1}"(arg1)
                 : "memory"
                 : "volatile");

    if res < 0 {
        Err(res.into())
    } else {
        Ok(res as usize)
    }
}
