#[repr(C)]
pub struct UtsName {
    /// sysname
    pub sysname: [u8; 65],
    /// nodename
    pub nodename: [u8; 65],
    /// release
    pub release: [u8; 65],
    /// version
    pub version: [u8; 65],
    /// machine
    pub machine: [u8; 65],
    /// domainname
    pub domainname: [u8; 65],
}

fn pad_str<const N: usize>(s: &str) -> [u8; N] {
    let mut arr = [0u8; N];
    let bytes = s.as_bytes();
    arr[..bytes.len()].copy_from_slice(bytes);
    arr
}

impl Default for UtsName {
    fn default() -> Self {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "x86_64")] {
                let machine = "x86_64";
            } else if #[cfg(target_arch = "aarch64")] {
                let machine = "aarch64";
            } else if #[cfg(target_arch = "riscv64")] {
                let machine = "riscv64";
            } else if #[cfg(target_arch = "loongarch64")] {
                let machine = "loongarch64";
            } else {
                let machine = "unknown";
            }
        }

        Self {
            sysname: pad_str("StarryOS"),
            nodename: pad_str("starry.next"),
            release: pad_str("0.1.0"),
            version: pad_str("ArceOS"),
            machine: pad_str(machine),
            domainname: pad_str("https://github.com/AsakuraMizu/starry-next/"),
        }
    }
}

pub fn sys_uname(name: *mut UtsName) -> i64 {
    let utsname = unsafe { &mut *name };
    *utsname = UtsName::default();
    0
}
