# Deployments and Operations

`atomrootfsinit` does not dictate how deployments are produced. Typical
strategies include:

- **Dedicated boot partition**: Kernel, modules, `atomrootfsinit`, and an
  updater live on a small partition. The real rootfs resides on another volume
  (possibly encrypted). This mimics the role of an initramfs without bundling
  one into the kernel image.
- **Split kernel/userspace partitions**: Keep kernel/modules separate from the
  rootfs but still run everything from block devices (ext4, XFS, etc.).
- **Single Btrfs volume**: Store multiple deployments as Btrfs subvolumes. An
  update creates a new subvolume, writes the rootfs, flips `default_subvol`, and
  updates `/etc/rdname` to point to the new release.

The shared goals for every layout:

- A single copy of the kernel/modules lives on disk.
- The bootloader stays untouched once initially configured.
- Systems that control their storage stack can disable initrd/initramfs support
  entirely.

## Requirements

- For `root=PARTUUID=...` to work reliably the kernel must include commit
  `758737d86f8a2d74c0fa9f8b2523fa7fd1e0d0aa` (or equivalent) so PARTUUIDs are
  exposed through `uevent`. The patch ships as
  `block-add-partition-uuid-into-uevent.patch`.
- Overlay at least `/lib/modules` (e.g. bind-mount it) whenever the kernel and
  the rootfs live in different deployments; otherwise module loading fails.
- Kernel updates are **not** bundled with rootfs updates when you use separate
  partitions. Plan to flash the kernel partition independently.
- Ensure the initial rootfs owns `/etc/rdname` (or accept that the current root
  will be reused). Updates simply replace that file with the name of the
  deployment to boot on the next restart.
- When booting through EFI + initramfs, consult `initramfs.md` for why
  `pivot_root` cannot be used and how `atomrootfsinit` falls back to `MS_MOVE`.

## Debugging and Recovery

- All diagnostics use `libc::printf`, so you see messages on the kernel console
  as long as `console=` is set.
- The optional `droptosh` Cargo feature keeps a rescue hatch: on fatal errors
  PID 1 tries to exec `/bin/sh` before sleeping and exiting.
- The `trace` feature emits a mount-by-mount log, invaluable when validating new
  `rdtab` entries.

## Operational Tips

- `/mnt` must exist *before* `atomrootfsinit` runs; typically it lives on the
  small “boot” rootfs alongside `/etc/rdname`.
- Remember to mount at least `/lib/modules` (or bind-mount it) if the kernel
  lives elsewhere than the rootfs you are switching to.
- When using Btrfs subvolumes, switch the default subvolume only after the new
  deployment is fully written and `/etc/rdtab` updated.
- Updates are atomic: write the new rootfs into a fresh directory or subvolume,
  adjust `/etc/rdname`, reboot, and the new deployment comes online without
  touching the bootloader.

