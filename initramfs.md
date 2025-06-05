# initramfs

On modern x86_64 and aarch64 machines UEFI is used to boot linux.

An efi executable contains the kernel, the initramfs, the cmdline, the splashscreen and optionally the signature.

While using initramfs the syscall sys_pivot_root is unavailable (will always fail with EINVAL) and therefore
this software includes special handling for this case.


