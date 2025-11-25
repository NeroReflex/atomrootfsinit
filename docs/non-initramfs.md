# Non-initramfs Deployments

Many embedded targets prefer to skip initramfs entirely and boot the kernel
directly from persistent storage. `atomrootfsinit` supports that mode without
extra tooling.

## Boot Expectations

- The kernel boots with `root=` pointing at a small “early root” partition that
  contains `atomrootfsinit`, `/etc/rdname`, `/etc/rdexec`, and an empty `/mnt`.
- `/etc/rdname` selects which directory under `/deployments` should be promoted
  to `/` on the next boot. Updating atomically is as simple as writing a new
  rootfs to `/deployments/<new>` and updating the file.
- After parsing `rdtab`, `atomrootfsinit` performs a classic `pivot_root`/`switch_root`
  because the environment is a real filesystem, not tmpfs.

## Storage Layout Recommendations

1. Reserve a small boot/maintenance partition that never changes automatically.
2. Keep `/lib/modules` either inside the staged deployment or mounted from the
   boot partition so kernel upgrades remain predictable.
3. When multiple deployments live on the same block device (e.g. Btrfs
   subvolumes) ensure the bootloader always points at the same kernel and init.

## Fallback Behavior

- If `/etc/rdname` is missing or empty the currently running root stays active.
- If `/etc/rdexec` is absent the kernel’s `init=` parameter is honored, falling
  back to `/sbin/init`.
- Fatal errors can optionally drop into `/bin/sh` when the `droptosh` feature is
  enabled, giving you a maintenance shell on the persistent root.

For initramfs-specific notes (where `pivot_root` is unavailable) see
`initramfs.md`.

