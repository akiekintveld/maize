use {
    crate::{
        frame::{Arc, Idx},
        sync::{Token, TokenCell},
        table::L2TableCap,
    },
    ::core::{
        fmt::{Debug, Formatter, Result as FmtResult},
        mem::{size_of, MaybeUninit},
    },
};

impl CallCap {
    pub fn new(frame_number: Idx, pc: usize, sp: usize, l2_table: L2TableCap) -> Option<Self> {
        let call = Call { pc, sp, l2_table };
        let call = TokenCell::new(call);
        let call = Arc::new(frame_number, call)?;
        Some(Self { call })
    }

    pub fn into_frame_number(self) -> Idx {
        self.call.into_raw()
    }
}

#[derive(Clone)]
pub struct CallCap {
    call: Arc<TokenCell<Call>>,
}

#[derive(Debug)]
struct Call {
    pc: usize,
    sp: usize,
    l2_table: L2TableCap,
}

impl ThreadCap {
    pub fn new(frame_number: Idx, context: Context, l2_table: L2TableCap) -> Option<Self> {
        let thread = Thread {
            context: Some(context),
            l2_table,
            call_stack: CallStack::empty(),
            exception_call: None,
        };
        let thread = TokenCell::new(thread);
        let thread = Arc::new(frame_number, thread)?;
        Some(Self { thread })
    }

    pub fn set_exception_call(&self, token: &mut Token, call: CallCap) {
        let thread = self.thread.borrow_mut(token);
        thread.exception_call = Some(call);
    }

    pub fn call_exception(&self, token: &mut Token) -> Option<()> {
        let thread = self.thread.borrow_mut(token);
        let exception_call = thread.exception_call.clone()?;
        self.call(token, &exception_call)
    }

    pub fn call(&self, token: &mut Token, call: &CallCap) -> Option<()> {
        let call = call.call.borrow(token);
        let pc = call.pc;
        let sp = call.sp;
        let l2_table = call.l2_table.clone();

        let thread = self.thread.borrow_mut(token);
        let context = thread.context.as_mut()?;
        thread.call_stack.push(Call {
            pc: context.pc,
            sp: context.sp,
            l2_table: thread.l2_table.clone(),
        })?;
        context.pc = pc;
        context.sp = sp;
        thread.l2_table = l2_table;
        Some(())
    }

    pub fn ret(&mut self, token: &mut Token) -> Option<()> {
        let thread = self.thread.borrow_mut(token);
        let context = thread.context.as_mut()?;
        let call = thread.call_stack.pop()?;
        context.pc = call.pc;
        context.sp = call.sp;
        thread.l2_table = call.l2_table;
        Some(())
    }

    pub fn context<'token>(&'token self, token: &'token Token) -> Option<&'token Context> {
        self.thread.borrow(token).context.as_ref()
    }

    pub fn context_mut<'token>(
        &'token self,
        token: &'token mut Token,
    ) -> Option<&'token mut Context> {
        self.thread.borrow_mut(token).context.as_mut()
    }

    pub fn l2_table<'token>(&'token self, token: &'token Token) -> &'token L2TableCap {
        &self.thread.borrow(token).l2_table
    }

    pub fn resume(&self, mut token: Token) -> Result<Token, Token> {
        let thread = self.thread.borrow_mut(&mut token);
        let mut context = if let Some(context) = thread.context.take() {
            context
        } else {
            return Err(token);
        };
        let l2_table = thread.l2_table.clone();
        l2_table.activate();
        token.release();

        let sstatus: u64;
        unsafe {
            asm!(
                "csrr {sstatus}, sstatus",
                sstatus = lateout(reg) sstatus,
            )
        }
        debug_assert_eq!(sstatus & SSTATUS_SPP_MASK, 0x0);

        unsafe {
            asm!(
                // Stash our context pointer. We're forced to use the last
                // register we restore to hold the context pointer, and then it
                // will load over itself.
                "csrw sscratch, a7",

                // Direct traps to come back to this function.
                "la t0, 1f",
                "csrw stvec, t0",

                // Save and restore integer registers.
                "ld ra, 0*{register_size}(a7)",
                "ld a6, 1*{register_size}(a7)",
                "csrw sepc, a6",
                "ld a6, 2*{register_size}(a7)",
                "sd sp, 2*{register_size}(a7)",
                "mv sp, a6",
                "ld a6, 3*{register_size}(a7)",
                "sd gp, 3*{register_size}(a7)",
                "mv gp, a6",
                "ld a6, 4*{register_size}(a7)",
                "sd tp, 4*{register_size}(a7)",
                "mv tp, a6",
                "ld t0, 5*{register_size}(a7)",
                "ld t1, 6*{register_size}(a7)",
                "ld t2, 7*{register_size}(a7)",
                "ld t3, 8*{register_size}(a7)",
                "ld t4, 9*{register_size}(a7)",
                "ld t5, 10*{register_size}(a7)",
                "ld t6, 11*{register_size}(a7)",
                "ld a6, 12*{register_size}(a7)",
                "sd s0, 12*{register_size}(a7)",
                "mv s0, a6",
                "ld a6, 13*{register_size}(a7)",
                "sd s1, 13*{register_size}(a7)",
                "mv s1, a6",
                "ld s2, 14*{register_size}(a7)",
                "ld s3, 15*{register_size}(a7)",
                "ld s4, 16*{register_size}(a7)",
                "ld s5, 17*{register_size}(a7)",
                "ld s6, 18*{register_size}(a7)",
                "ld s7, 19*{register_size}(a7)",
                "ld s8, 20*{register_size}(a7)",
                "ld s9, 21*{register_size}(a7)",
                "ld s10, 22*{register_size}(a7)",
                "ld s11, 23*{register_size}(a7)",
                "ld a0, 24*{register_size}(a7)",
                "ld a1, 25*{register_size}(a7)",
                "ld a2, 26*{register_size}(a7)",
                "ld a3, 27*{register_size}(a7)",
                "ld a4, 28*{register_size}(a7)",
                "ld a5, 29*{register_size}(a7)",
                "ld a6, 30*{register_size}(a7)",
                "ld a7, 31*{register_size}(a7)",

                // Jump back to the saved program counter in user mode.
                "sret",

                // Note that trap handlers must be aligned on 4-byte boundaries.
                ".align 0x4",
                "1:",

                // Stash trap stack pointer and retrieve ours.
                "csrrw a7, sscratch, sp",

                // Save and restore integer registers.
                "sd ra, 0*{register_size}(a7)",
                "csrr ra, sepc",
                "sd ra, 1*{register_size}(a7)",
                "ld ra, 2*{register_size}(a7)",
                "sd sp, 2*{register_size}(a7)",
                "mv sp, ra",
                "ld ra, 3*{register_size}(a7)",
                "sd gp, 3*{register_size}(a7)",
                "mv gp, ra",
                "ld ra, 4*{register_size}(a7)",
                "sd tp, 4*{register_size}(a7)",
                "mv tp, ra",
                "sd t0, 5*{register_size}(a7)",
                "sd t1, 6*{register_size}(a7)",
                "sd t2, 7*{register_size}(a7)",
                "sd t3, 8*{register_size}(a7)",
                "sd t4, 9*{register_size}(a7)",
                "sd t5, 10*{register_size}(a7)",
                "sd t6, 11*{register_size}(a7)",
                "ld ra, 12*{register_size}(a7)",
                "sd s0, 12*{register_size}(a7)",
                "mv s0, ra",
                "ld ra, 13*{register_size}(a7)",
                "sd s1, 13*{register_size}(a7)",
                "mv s1, ra",
                "sd s2, 14*{register_size}(a7)",
                "sd s3, 15*{register_size}(a7)",
                "sd s4, 16*{register_size}(a7)",
                "sd s5, 17*{register_size}(a7)",
                "sd s6, 18*{register_size}(a7)",
                "sd s7, 19*{register_size}(a7)",
                "sd s8, 20*{register_size}(a7)",
                "sd s9, 21*{register_size}(a7)",
                "sd s10, 22*{register_size}(a7)",
                "sd s11, 23*{register_size}(a7)",
                "sd a0, 24*{register_size}(a7)",
                "sd a1, 25*{register_size}(a7)",
                "sd a2, 26*{register_size}(a7)",
                "sd a3, 27*{register_size}(a7)",
                "sd a4, 28*{register_size}(a7)",
                "sd a5, 29*{register_size}(a7)",
                "sd a6, 30*{register_size}(a7)",
                "csrr ra, sscratch",
                "sd ra, 31*{register_size}(a7)",

                register_size = const size_of::<usize>(),

                out("s2") _,
                out("s3") _,
                out("s4") _,
                out("s5") _,
                out("s6") _,
                out("s7") _,
                out("s8") _,
                out("s9") _,
                out("s10") _,
                out("s11") _,
                in("a7") &mut context,
                clobber_abi("C"),
            );
        }

        unsafe {
            asm!(
                "csrw stvec, {stvec}",
                stvec = in(reg) supervisor_trap,
            )
        }

        let mut token = Token::acquire();
        let thread = self.thread.borrow_mut(&mut token);
        thread.context = Some(context);

        Ok(token)
    }

    pub fn into_frame_number(self) -> Idx {
        self.thread.into_raw()
    }
}

#[derive(Clone)]
pub struct ThreadCap {
    thread: Arc<TokenCell<Thread>>,
}

struct Thread {
    context: Option<Context>,
    l2_table: L2TableCap,
    call_stack: CallStack,
    exception_call: Option<CallCap>,
}

impl CallStack {
    const MAX_DEPTH: usize = 8;

    pub fn empty() -> Self {
        const CALL_INIT: MaybeUninit<Call> = MaybeUninit::uninit();
        Self {
            calls: [CALL_INIT; Self::MAX_DEPTH],
            depth: 0,
        }
    }

    pub fn push(&mut self, call: Call) -> Option<&mut Call> {
        let c = self.calls.get_mut(self.depth)?;
        let c = c.write(call);
        self.depth += 1;
        Some(c)
    }

    pub fn pop(&mut self) -> Option<Call> {
        let index = self.depth.checked_sub(1)?;
        let c = &mut self.calls[index];
        // SAFETY: All elements prior to `self.depth` are initialized, and
        // subsequently this element will be considered uninitialized.
        let call = unsafe { c.as_ptr().read() };
        self.depth = index;
        Some(call)
    }
}

struct CallStack {
    calls: [MaybeUninit<Call>; Self::MAX_DEPTH],
    depth: usize,
}

/// Enters execution of the kernel upon a trap from supervisor mode.
///
/// # Safety
/// Must be called by a trap from supervisor mode.
#[naked]
#[repr(align(0x4))]
pub unsafe extern "C" fn supervisor_trap() -> ! {
    unsafe extern "C" fn handle_supervisor_trap(context: &Context) -> ! {
        let scause: usize;
        let stval: usize;

        unsafe {
            asm!(
                "csrr {scause}, scause",
                "csrr {stval}, stval",
                scause = lateout(reg) scause,
                stval = lateout(reg) stval,
            )
        }

        panic!(
            "Unexpected supervisor trap with context: {:?}, scause: {:#x}, stval: {:#x}",
            context, scause, stval,
        );
    }

    // SAFETY: We entered via a trap. See below for the reasoning behind each
    // block of instructions.
    unsafe {
        asm!(
            // Stash trap stack pointer.
            "csrw sscratch, sp",

            // Make space to save the context.
            "addi sp, sp, -{context_size}",

            // Save and restore integer registers.
            "sd ra, 0*{register_size}(sp)",
            "csrr ra, sepc",
            "sd ra, 1*{register_size}(sp)",
            "csrr ra, sscratch",
            "sd ra, 2*{register_size}(sp)",
            "sd gp, 3*{register_size}(sp)",
            "sd tp, 4*{register_size}(sp)",
            "sd t0, 5*{register_size}(sp)",
            "sd t1, 6*{register_size}(sp)",
            "sd t2, 7*{register_size}(sp)",
            "sd t3, 8*{register_size}(sp)",
            "sd t4, 9*{register_size}(sp)",
            "sd t5, 10*{register_size}(sp)",
            "sd t6, 11*{register_size}(sp)",
            "sd s0, 12*{register_size}(sp)",
            "sd s1, 13*{register_size}(sp)",
            "sd s2, 14*{register_size}(sp)",
            "sd s3, 15*{register_size}(sp)",
            "sd s4, 16*{register_size}(sp)",
            "sd s5, 17*{register_size}(sp)",
            "sd s6, 18*{register_size}(sp)",
            "sd s7, 19*{register_size}(sp)",
            "sd s8, 20*{register_size}(sp)",
            "sd s9, 21*{register_size}(sp)",
            "sd s10, 22*{register_size}(sp)",
            "sd s11, 23*{register_size}(sp)",
            "sd a0, 24*{register_size}(sp)",
            "mv a0, sp",
            "sd a1, 25*{register_size}(sp)",
            "sd a2, 26*{register_size}(sp)",
            "sd a3, 27*{register_size}(sp)",
            "sd a4, 28*{register_size}(sp)",
            "sd a5, 29*{register_size}(sp)",
            "sd a6, 30*{register_size}(sp)",
            "sd a7, 31*{register_size}(sp)",

            // Align the stack since we could've trapped from anywhere.
            "andi sp, sp, -0x10",

            // Jump into Rust to handle the trap.
            "j {handle_supervisor_trap}",

            context_size = const size_of::<Context>(),
            register_size = const size_of::<usize>(),
            handle_supervisor_trap = sym handle_supervisor_trap,

            options(noreturn),
        )
    }
}

pub const SSTATUS_SPP_MASK: u64 = 0x100u64;

/// General purpose register context for a hart.
#[repr(C, align(16))]
#[derive(Clone, Default)]
pub struct Context {
    pub ra: usize,
    pub pc: usize,
    pub sp: usize,
    pub gp: usize,
    pub tp: usize,
    pub t: [usize; 7],
    pub s: [usize; 12],
    pub a: [usize; 8],
}

impl Debug for Context {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("Context")
            .field("ra", &format_args!("{:#x}", self.ra))
            .field("pc", &format_args!("{:#x}", self.pc))
            .field("sp", &format_args!("{:#x}", self.sp))
            .field("gp", &format_args!("{:#x}", self.gp))
            .field("tp", &format_args!("{:#x}", self.tp))
            .field("t0", &format_args!("{:#x}", self.t[0]))
            .field("t1", &format_args!("{:#x}", self.t[1]))
            .field("t2", &format_args!("{:#x}", self.t[2]))
            .field("t3", &format_args!("{:#x}", self.t[3]))
            .field("t4", &format_args!("{:#x}", self.t[4]))
            .field("t5", &format_args!("{:#x}", self.t[5]))
            .field("t6", &format_args!("{:#x}", self.t[6]))
            .field("s0", &format_args!("{:#x}", self.s[0]))
            .field("s1", &format_args!("{:#x}", self.s[1]))
            .field("s2", &format_args!("{:#x}", self.s[2]))
            .field("s3", &format_args!("{:#x}", self.s[3]))
            .field("s4", &format_args!("{:#x}", self.s[4]))
            .field("s5", &format_args!("{:#x}", self.s[5]))
            .field("s6", &format_args!("{:#x}", self.s[6]))
            .field("s7", &format_args!("{:#x}", self.s[7]))
            .field("s8", &format_args!("{:#x}", self.s[8]))
            .field("s9", &format_args!("{:#x}", self.s[9]))
            .field("s10", &format_args!("{:#x}", self.s[10]))
            .field("s11", &format_args!("{:#x}", self.s[11]))
            .field("a0", &format_args!("{:#x}", self.a[0]))
            .field("a1", &format_args!("{:#x}", self.a[1]))
            .field("a2", &format_args!("{:#x}", self.a[2]))
            .field("a3", &format_args!("{:#x}", self.a[3]))
            .field("a4", &format_args!("{:#x}", self.a[4]))
            .field("a5", &format_args!("{:#x}", self.a[5]))
            .field("a6", &format_args!("{:#x}", self.a[6]))
            .field("a7", &format_args!("{:#x}", self.a[7]))
            .finish()
    }
}
