# atomrootfsinit

`atomrootfsinit` is a tiny Rust `init` whose only job is to mount the *real*
root filesystem, hand control to your preferred init system, and make atomic
rootfs updates practical on devices where an initramfs is either unavailable or
unwanted.  By deferring every destructive action to the next boot it allows you
to stage updates safely while keeping PID 1 as small and auditable as possible.

The project answers three recurring issues in embedded Linux deployments:
- `systemd` (and other inits) expect early write access to files such as
  `/etc/machine-id` before `/etc/fstab` is honored.
- Swapping to a freshly written deployment should not require risky A/B layouts
  or bootloader edits.
- Many targets can drop heavy initramfs images altogether once a smarter PID 1
  can assemble the final rootfs.

## Why?

`atomrootfsinit` is born out of my own frustration with the boot process,
because the state of the art for embedded Linux updates often felt clumsy:
heavy initramfs images, fragile bootloader logic, and init systems that assume
they can already write to `/etc`. `atomrootfsinit` exists to remove that
frustration by keeping PID 1 laser-focused on mounting the correct deployment
and getting out of the way.

## High-level boot flow

1. `atomrootfsinit` is executed as PID 1 (either directly via `init=` or
   through a tiny initrd/initramfs that execs it).
2. Every shared mount is made private so later `MS_MOVE`/`pivot_root` calls
   succeed.
3. `/etc/rdname` is parsed. If it contains `my-release`, the directory
   `/deployments/my-release` is bind-mounted over `/mnt`. If the file is
   missing or empty the running root is bind-mounted instead. This directory is
   the *staged* rootfs.
4. `/mnt/etc/rdtab` is loaded and parsed. The syntax mirrors `/etc/fstab`, but
   it lives inside the staged root because it describes *that* system.
5. `/proc` and any mounts described as `sysfs` or `devtmpfs` are mounted first;
   this guarantees that PARTUUID lookups work even in minimal environments.
6. If the kernel `root=` parameter is a `PARTUUID=...`, the matching block
   device is located under sysfs and the placeholder source `rootdev` inside
   `rdtab` is replaced with the resolved device node.
7. All remaining entries from `rdtab` are mounted, producing a complete rootfs
   below `/mnt` (or an alternate target specified for `rootdev`).
8. `/etc/rdexec` (on the *current* early root) selects the final init binary.
   When absent, `init=` from the kernel command line is honored, otherwise
   `/sbin/init` is used.
9. The tool detects whether the current environment is an initramfs by checking
   `/proc/mounts`. It then performs either a `MS_MOVE + chroot` (initramfs) or
   a classic `pivot_root` (initrd/real FS) via `switch_root`.
10. The chosen init is `execve`'d, replacing `atomrootfsinit` while preserving
    PID 1.

Additional background about the initramfs path and EFI booting can be found in
[`initramfs.md`](./initramfs.md).

## Runtime contract

`atomrootfsinit` expects a few files and directories to exist:

| Path | Location | Purpose |
| --- | --- | --- |
| `/mnt` | early root | Target directory where the staged rootfs is mounted. Must exist before boot. |
| `/etc/rdname` | early root | Optional text file whose first (trimmed) line matches a directory under `/deployments`. Controls which deployment becomes the new root. |
| `/deployments/<name>` | early root | Directory containing the staged rootfs that should become `/`. |
| `/mnt/etc/rdtab` | staged root | fstab-like file that lists every mount needed by the final system, including `rootdev`. |
| `/etc/rdexec` | early root | Optional path (UTF-8, newline trimmed) to the init binary that should be `execve`'d after `switch_root`. |

If `/etc/rdname` is missing, the currently running rootfs is reused. If
`/etc/rdexec` is missing, the kernel’s `init=` parameter is used, falling back
to `/sbin/init`.

### `rdtab` format

`rdtab` matches the traditional six-column `/etc/fstab` layout:

```
<src> <target> <fstype> <options> <dump> <pass>
```

Key extensions:
- Use `rootdev` as the `<src>` to indicate “mount whatever the kernel passed as
  `root=`”. When `root=` is `PARTUUID=...`, the PARTUUID is resolved via
  sysfs/devtmpfs before the mount is attempted.
- The `<options>` column accepts both standard mount flags (`ro,noexec,...`) and
  filesystem-specific comma-separated data, exactly like `/etc/fstab`.
- Lines may contain `# comments`.

Example `rdtab`:

```
sysfs /sys sysfs rw 0 0
devtmpfs /dev devtmpfs rw,nosuid,noexec 0 0
rootdev / newroot ext4 rw,nodev,noexec 0 1
tmpfs /run tmpfs rw,nodev,nosuid,size=64M 0 0
proc /proc proc rw,nosuid,nodev,noexec 0 0
```

In the example above, the third line mounts the device named by `root=` (for
instance `/dev/mmcblk0p3` or the device resolved from `PARTUUID=`) onto
`/newroot`. If you want the final root to live somewhere else (e.g. `/mnt`,
`/sysroot`), simply make `rootdev` target that directory.

### Deployment layouts

`atomrootfsinit` does not dictate how deployments are produced. Typical
strategies include:

- **Dedicated boot partition**: Kernel, modules, `atomrootfsinit`, and an
  updater live on a small partition. The real rootfs resides on another volume
  (possibly encrypted). This mimics the role of an initramfs without actually
  bundling one into the kernel image.
- **Split kernel/userspace partitions**: Keep kernel/modules separate from the
  rootfs but still run everything from block devices (ext4, XFS, etc.).
- **Single Btrfs volume**: Store multiple deployments as Btrfs subvolumes. An
  update creates a new subvolume, writes the rootfs, flips `default_subvol`,
  and updates `/etc/rdname` to point to the new release.

All options share the same goals:
- A single copy of the kernel/modules is present on disk.
- The bootloader never needs to change once the initial configuration is done.
- Systems that control their storage stack can drop initrd/initramfs support
  entirely.

### Build and install

```
cargo build --release
```

For static binaries (recommended for initramfs use) run:

```
./staticbuild.sh
```

The resulting `target/<triple>/release/atomrootfsinit` can be copied to `/sbin`
or bundled into an initrd. When packaged as a Debian artifact the provided
`Cargo.toml` metadata installs it under `usr/bin/`.

## Requirements

- For `root=PARTUUID=...` to work reliably the kernel must include commit
  `758737d86f8a2d74c0fa9f8b2523fa7fd1e0d0aa` (or equivalent) so PARTUUIDs are
  exposed through `uevent`. The patch is shipped as
  [`block-add-partition-uuid-into-uevent.patch`](./block-add-partition-uuid-into-uevent.patch).
- _REMEMBER TO OVERLAY AT LEAST THE MODULES DIRECTORY_ (for example bind-mount
  `/lib/modules` from the kernel partition) whenever the kernel and the rootfs
  live in different deployments; otherwise the new system will boot without
  loadable modules.
- Kernel updates are **not** bundled with rootfs updates when you use separate
  partitions. Plan to flash the kernel partition independently.
- Ensure the initial rootfs owns `/etc/rdname` (or accept that the current root
  will be reused). Updates simply replace that file with the name of the
  deployment to boot on the next restart.
- When booting through EFI + initramfs, read the short notes in
  [`initramfs.md`](./initramfs.md) about why `pivot_root` cannot be used and how
  `atomrootfsinit` falls back to `MS_MOVE`.

### Debugging and recovery

- All diagnostics are printed with `libc::printf`, so they appear on the kernel
  console as long as `console=` is set.
- The optional `droptosh` cargo feature keeps a rescue hatch: on fatal errors
  PID 1 tries to exec `/bin/sh` before sleeping and exiting.
- The `trace` feature emits a mount-by-mount log, useful when validating new
  `rdtab` entries.

### Tips and gotchas

- `/mnt` must exist *before* `atomrootfsinit` runs; typically it lives on the
  small “boot” rootfs alongside `/etc/rdname`.
- Remember to mount at least `/lib/modules` (or bind-mount it) if the kernel
  lives elsewhere than the rootfs you are switching to.
- When using Btrfs subvolumes, switch the default subvolume only after the new
  deployment is fully written and `/etc/rdtab` updated.
- Updates are atomic: write the new rootfs into a fresh directory or subvolume,
  adjust `/etc/rdname`, reboot, and the new deployment comes online without
  touching the bootloader.

## License

`atomrootfsinit` is licensed under GPL-2.0-or-later. See [`LICENSE.md`](./LICENSE.md)
for details.

## Requirements

`atomrootfsinit` supports root=PARTUUID=00000000000000000000 only when the commit 758737d86f8a2d74c0fa9f8b2523fa7fd1e0d0aa is present in the booted kernel:
the patch is available in the file block-add-partition-uuid-into-uevent.patch
