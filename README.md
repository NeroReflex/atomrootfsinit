# atomrootfsinit

`atomrootfsinit` is a tiny Rust `init` whose only job is to mount the *real*
root filesystem, hand control to your preferred init system, and make atomic
rootfs updates practical on devices where an initramfs is either unavailable or
unwanted. By deferring destructive actions to the next boot it lets you stage
updates safely while keeping PID 1 as small and auditable as possible.

The project answers three recurring pain points in embedded deployments:

- `systemd` (and other inits) expect early write access before `/etc/fstab` is
  honored.
- Swapping to a freshly written deployment should not require risky A/B layouts
  or bootloader edits.
- Many targets can drop heavy initramfs images once a smarter PID 1 assembles
  the final rootfs.

## Why?

`atomrootfsinit` is born out of my own frustration with the boot process,
because the state of the art for embedded Linux updates often felt clumsy:
heavy initramfs images, fragile bootloader logic, and init systems that assume
they can already write to `/etc`. `atomrootfsinit` exists to remove that
frustration by keeping PID 1 laser-focused on mounting the correct deployment
and getting out of the way.

## Documentation Map

- [Boot flow](./docs/boot-flow.md) – every step from PID 1 to handing off init.
- [Runtime contract & `rdtab` format](./docs/runtime.md) – required files and
  how to express mounts.
- [Deployments & operations](./docs/deployments.md) – storage layouts,
  requirements, debugging, and operational tips.
- [Initramfs notes](./initramfs.md) – EFI behavior and why `pivot_root` is
  unavailable there.
- [Non-initramfs guide](./docs/non-initramfs.md) – using classic `pivot_root`
  on persistent roots.
- [Usage examples](./docs/examples.rdtab) – ready-to-copy `rdtab` scenarios.

## Build and Install

```
cargo build --release
```

For static binaries (recommended for initramfs use):

```
./staticbuild.sh
```

The resulting `target/<triple>/release/atomrootfsinit` can be copied to `/sbin`
or bundled into an initrd. When packaged as a Debian artifact the provided
`Cargo.toml` installs it under `usr/bin/`.

## Requirements

`atomrootfsinit` supports root=PARTUUID=00000000000000000000 only when the commit 758737d86f8a2d74c0fa9f8b2523fa7fd1e0d0aa is present in the booted kernel:
the patch is available in the file block-add-partition-uuid-into-uevent.patch

## License

`atomrootfsinit` is licensed under GPL-2.0-or-later. See [`LICENSE.md`](./LICENSE.md)
for details.
