use {
    crate::{
        frame::{Arc, Idx},
        sync::{Token, TokenCell},
        table::L2TableCap,
    },
    ::core::{
        fmt::{Debug, Formatter, Result as FmtResult},
        mem::MaybeUninit,
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

    pub fn resume(&self, mut token: Token) -> Result<(Token, u64, u64), Token> {
        let thread = self.thread.borrow_mut(&mut token);
        let mut context = if let Some(context) = thread.context.take() {
            context
        } else {
            return Err(token);
        };
        let l2_table = thread.l2_table.clone();
        l2_table.activate();
        token.release();

        let (scause, stval) = unsafe { crate::plat::resume(&mut context) };

        let mut token = Token::acquire();
        let thread = self.thread.borrow_mut(&mut token);
        thread.context = Some(context);

        Ok((token, scause, stval))
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

pub const SSTATUS_SPP_MASK: u64 = 0x100u64;

/// General purpose register context for a hart.
#[repr(C)]
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
