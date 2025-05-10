# atomrootfsinit

This tool mounts the rootfs and calls the init program on the mounted system.

This tool is designed to solve three problems:
1. systemd expect to be able to write /etc/machine-id before /etc/fstab is even considered
2. Boot the new rootfs after an update has been written
3. End the need for an initramfs: initramfs is just a leftover from a long-gone era

## Why?

Because I was frustrated about the state of the art in embedded linux world.

## How?

Start this software either symlinking it to /sbin/init to use kernel parameters *init=* or *initrd=* (in case you want to use this software on an initramfs).

When this software starts it reads /etc/rdname: if that file is found it will read the first line (the *name* of the release) and mount /deployments/*name* to /mnt. If that file is not found, or the first line is empty a bind mount of / to /mnt will be performed.

Regardless of what happened on the first step the /mnt directory (that you _MUST_ create on the root of the booted filesystem) will be a mountpoint containing your new rootfs.

The second step is reading the file /mnt/etc/rdtab and mounting everything described in there.

The third step is reading the file /mnt/etc/rdinit: this is the executable that will called as part of the switch_root operation:
usually this file is systemd.

The last step is performing the switch_root operation that will start the real init replacing the current process (keeping the PID number).

## Further Exaplaination

This software is used to update the running linux without affecting it in any way: everything will happen at the next boot!

You can choose between these layouts:
- having a separate kernel+modules+atomrootfsinit partition that will be the equivalent of initramfs in case you don't want to link every required modules into vmlinux or your primary root is encrypted
- having a non-btrfs rootfs that has kernel+modules and userspace separate
- having a big btrfs rootfs that holds everything

Each one of these comes with pros and cons, but both reach the design goal:
- only one copy of kernel and modules exists on the system
- the bootloader is never touched again after the first configuration
- you can remove initrd support from the kernel

In the first two cases you will need to:
- _REMEMBER TO OVERLAY AT LEAST THE MODULES DIRECTORY_
- Remember that kernel updates are not shipped with rootfs updates and the kernel is to be updated separately

In case of a btrfs partition:
- You write the rootfs in a subvolume, get its ID and change the default subvolume of rootfs, also uupdating the kernel with the rest of the system
- You can use btrfs send and receive instead of unpacking a .tar rootfs
