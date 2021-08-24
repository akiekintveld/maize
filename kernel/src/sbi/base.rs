use {
    crate::sbi::call,
    ::core::{
        any::type_name,
        fmt::{Debug, Display, Formatter, Result},
        num::NonZeroUsize,
    },
};

pub struct SpecVersion(usize);

impl SpecVersion {
    pub fn major(&self) -> usize {
        // Major number is in bits [24..31]
        (self.0 >> 24) & 0x7f
    }

    pub fn minor(&self) -> usize {
        // Minor number is in bits [0..23]
        self.0 & 0xffffff
    }
}

impl Display for SpecVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}.{}", self.major(), self.minor())
    }
}

impl Debug for SpecVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct(type_name::<Self>())
            .field("major", &self.major())
            .field("minor", &self.minor())
            .finish()
    }
}

pub enum ImplId {
    Bbl,
    OpenSbi,
    Xvisor,
    Kvm,
    RustSbi,
    Diosix,
    Unknown(usize),
}

impl Display for ImplId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Bbl => write!(f, "Berkley Boot Loader"),
            Self::OpenSbi => write!(f, "OpenSBI"),
            Self::Xvisor => write!(f, "Xvisor"),
            Self::Kvm => write!(f, "KVM"),
            Self::RustSbi => write!(f, "RustSBI"),
            Self::Diosix => write!(f, "Diosix"),
            Self::Unknown(id) => write!(f, "unknown SBI implementation (ID: {})", id),
        }
    }
}

pub enum ExtAvail {
    Unavailable,
    Available(NonZeroUsize),
}

pub struct VendorId(usize);

impl VendorId {
    pub fn bank(&self) -> usize {
        // Bank is in bits [7..31]
        (self.0 >> 6) & 0x1ffffff
    }

    pub fn offset(&self) -> usize {
        // Offset is in bits [0..6]
        self.0 & 0x3f
    }
}

impl Display for VendorId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}:{}", self.bank(), self.offset())
    }
}

impl Debug for VendorId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct(type_name::<Self>())
            .field("bank", &self.bank())
            .field("offset", &self.offset())
            .finish()
    }
}

pub const EID: u32 = 0x10;

const EXPECT: &'static str =
    "The base extension functions must be supported by all implementations.";

pub fn spec_version() -> SpecVersion {
    let res = unsafe { call(EID, 0x0, 0, 0, 0, 0, 0, 0) };
    let ver = res.expect(EXPECT);
    SpecVersion(ver)
}

pub fn impl_id() -> ImplId {
    let res = unsafe { call(EID, 0x1, 0, 0, 0, 0, 0, 0) };
    let id = res.expect(EXPECT);
    match id {
        0 => ImplId::Bbl,
        1 => ImplId::OpenSbi,
        2 => ImplId::Xvisor,
        3 => ImplId::Kvm,
        4 => ImplId::RustSbi,
        5 => ImplId::Diosix,
        _ => ImplId::Unknown(id),
    }
}

pub fn impl_version() -> usize {
    let res = unsafe { call(EID, 0x2, 0, 0, 0, 0, 0, 0) };
    res.expect(EXPECT)
}

pub fn probe_extension(eid: u32) -> ExtAvail {
    let res = unsafe { call(EID, 0x3, eid as usize, 0, 0, 0, 0, 0) };
    let avail = res.expect(EXPECT);
    if let Some(avail) = NonZeroUsize::new(avail) {
        ExtAvail::Available(avail)
    } else {
        ExtAvail::Unavailable
    }
}

pub fn machine_vendor_id() -> VendorId {
    let res = unsafe { call(EID, 0x4, 0, 0, 0, 0, 0, 0) };
    let id = res.expect(EXPECT);
    VendorId(id)
}

pub fn machine_arch_id() -> usize {
    let res = unsafe { call(EID, 0x5, 0, 0, 0, 0, 0, 0) };
    res.expect(EXPECT)
}

pub fn machine_impl_id() -> usize {
    let res = unsafe { call(EID, 0x6, 0, 0, 0, 0, 0, 0) };
    res.expect(EXPECT)
}
