Maize is a capability-based microkernel written in Rust for RISC-V.

Maize draws inspiration for its design and architecture from other
microkernels and capability-based systems. In particular, [seL4][0],
and [Composite][1] were both significant influences.

Provides capability-based user-controlled allocation of physical memory 
(including for kernel objects), address space manipulation, thread scheduling, 
interrupt management, and thread migration for basic hart-local IPC. The focus 
is on providing only the minimum functionality necessary to securely build 
complex systems on top.

**Maize should absolutely not be considered production-ready and makes no
attempt at stability in its API, ABI, or basic functionality.** It is first
and foremost a side-project I started in order to teach myself some operating 
system design, get some practice using Rust in new ways, and to become familiar 
with RISC-V. However, I have made reasonable effort to write idiomatic code, 
design safe interfaces, and provide decent documentation.

# Why a capability-based microkernel?
I appreciate the elegance of keeping the trusted computing base lean and 
permitting flexible system designs on top. It also means the task is far less
daunting, which has kept me motivated.

# Why Rust?
I find it painful to code without a borrow checker or strong type system now,
and Rust's `no_std` provides a suprisingly pleasant and complete programming 
environment. I haven't been able to use it at my day job, so this has also been 
a great opportunity to keep my skills sharp.

# Why RISC-V?
If I'm going to be writing assembly and reading manuals in my free time, I 
figured I should learn a new ISA. It turned out to be a nice target for a small 
project that doesn't need the complexity of more mature architectures. Maybe 
someday I'll do a port to ARM.

# Rust Safety for Kernels
A kernel is a bit of a unique target when it comes to Rust's concepts of
safety. In particular, it is helpful to clearly define what thread and
memory safety mean for a program that manages many address spaces and
threads of execution, including its own.

Our kernel is non-preemptible, and makes use of a single kernel stack per
hardware thread (hart in RISC-V parlance). It shares a single identical (modulo
the user mode half) address space across all harts. Aside from early boot, it
otherwise keeps the mappings of its inter-hart shared program segments fixed
and at least as permissive as necessary for regular process execution. This
means we can roughly treat harts like threads (albeit with static lifetimes)
and the kernel like a process for Rust safety purposes.

For this model to be safe, we need to ensure that user mode can never read or
write kernel memory, and vice versa (we do not permit supervisor mode to access
user mode memory, as we only use registers for all trap handling including
environment calls). Most of the responsibility for this falls on the early boot,
frame management, and address translation and protection code. The specific
safety requirements and guarantees for each are documented more explicitly
in their respective modules. **This means we can rely on `Send`, `Sync`, and the
rest of the Rust type and lifetime system to ensure thread and memory safety as 
long as we uphold these assumptions and the rest of Rust's safety rules in all 
of our `unsafe` blocks.**

It is worth noting, however, that the kernel only ensures its own thread or
memory safety. User mode programs written in Rust or any other language
can—quite easily—violate their own thread or memory safety. In the longer term, 
we should provide crates to provide a safer user mode interface to the kernel.

Note that this means a single kernel image cannot be used across heterogenous
harts that have different views of memory or support different feature sets,
as this breaks the single process model (see Embedded WG [RFC 419][2]). For
example, we cannot take advantage of both the S7 and U74 cores on the SiFive
Freedom U740 SoC with a single kernel image. In the future, we may consider a
multikernel design that models each group of homogenous harts as a separate
"processes" and uses message passing between them. This would also provide
benefits if we decided to group harts based on the cache geometry to reduce
coherence overhead.

[0]: https://sel4.systems
[1]: https://composite.seas.gwu.edu
[2]: https://github.com/rust-embedded/wg/blob/master/rfcs/0419-multi-core-soundness.md
