# Boot Flow

`atomrootfsinit` keeps PID 1 intentionally small. The entire boot sequence fits
on a single page so you can audit it easily:

1. `atomrootfsinit` runs as PID 1 (either via `init=` or a tiny initrd that
   execs it).
2. All shared mounts are made private so later `MS_MOVE`/`pivot_root` calls do
   not fail.
3. `/etc/rdname` is parsed. If it contains `my-release`, the directory
   `/deployments/my-release` is bind-mounted over `/mnt`. When the file is
   missing or empty, the currently running root becomes the staged root.
4. `/mnt/etc/rdtab` is loaded. The syntax mirrors `/etc/fstab` but describes the
   staged system.
5. `/proc` plus any `sysfs` or `devtmpfs` entries are mounted first so that
   PARTUUID lookups work, even on minimal early roots.
6. If the kernel `root=` parameter is `PARTUUID=...`, the matching block device
   is located under sysfs and the placeholder source `rootdev` in `rdtab` is
   replaced with the resolved device node.
7. All remaining `rdtab` entries are mounted, producing a complete rootfs below
   `/mnt` (or whichever target `rootdev` used).
8. `/etc/rdexec` (on the early root) selects the final init binary. If it is
   absent, the kernel `init=` parameter wins, otherwise `/sbin/init` is used.
9. The environment is inspected: on initramfs `MS_MOVE + chroot` is used, on
   initrd/real filesystems the classic `pivot_root` path via `switch_root` is
   taken.
10. The chosen init is `execve`'d, preserving PID 1.

Additional background for the initramfs path and EFI booting lives in
`initramfs.md`. Guidance for non-initramfs deployments is documented in
`non-initramfs.md`.

