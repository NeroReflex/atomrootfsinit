# Runtime Contract

`atomrootfsinit` expects a few files and directories to exist before PID 1 runs:

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

## `rdtab` Format

`rdtab` matches the traditional six-column `/etc/fstab` layout:

```
<src> <target> <fstype> <options> <dump> <pass>
```

Key extensions:

- Use `rootdev` as the `<src>` to indicate “mount whatever the kernel passed as
  `root=`”. `/proc/cmdline` must be available before `rootdev` is processed, so
  ensure `/proc` is mounted early (see `docs/boot-flow.md`). When `root=` is
  `PARTUUID=...`, the PARTUUID is resolved via sysfs/devtmpfs before the mount
  is attempted.
- The `<options>` column accepts both standard mount flags (`ro,noexec,...`) and
  filesystem-specific comma-separated data, exactly like `/etc/fstab`.
- Lines may contain `# comments`.

See `docs/examples.rdtab` for annotated usage patterns.

