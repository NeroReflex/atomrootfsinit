Name:           atomrootfsinit
Version:        %{version}
Release:        1%{?dist}
Summary:        atomrootfsinit - a small init that mounts the rootfs and transfer control

License:        GPLv2
URL:            https://github.com/neroreflex/atomrootfsinit
BuildRequires:  cargo, gcc, clang, pkgconfig, make
# Provide explicit runtime Requires to avoid empty macro expansion in some rpmbuild setups
Recommends:     systemd

%description
atomrootfsinit is an init service that mounts the root filesystem and transfer control over some other init.

%prep
# No source tarball; build directly from the checked-out tree.
# rpmbuild will be invoked with `_sourcedir` pointing at the repo.

%build
export CARGO_HOME="$HOME/.cargo" || true
export PATH="$HOME/.cargo/bin:$PATH"
# Build all binaries in release mode and place artifacts under the checked-out
# repository `target/` directory so `%install` can find `target/release/...`.
cargo build --release --manifest-path "%{_sourcedir}/Cargo.toml" --target-dir "%{_sourcedir}/target"

%install
ls -lah .
rm -rf %{buildroot}
mkdir -p %{buildroot}/usr/bin
install -m 755 "%{_sourcedir}/target/release/atomrootfsinit" %{buildroot}/usr/bin/atomrootfsinit

# Install documentation and license from the repository so %doc/%license work
mkdir -p %{buildroot}/usr/share/doc/atomrootfsinit
if [ -f %{_sourcedir}/README.md ]; then
	install -m 644 %{_sourcedir}/README.md %{buildroot}/usr/share/doc/atomrootfsinit/README.md
else
	echo "README.md missing in %{_sourcedir}; cannot populate %doc" >&2
	exit 1
fi

mkdir -p %{buildroot}/usr/share/licenses/atomrootfsinit
if [ -f %{_sourcedir}/LICENSE.md ]; then
	install -m 644 %{_sourcedir}/LICENSE.md %{buildroot}/usr/share/licenses/atomrootfsinit/LICENSE.md
else
	echo "LICENSE.md missing in %{_sourcedir}; cannot populate %license" >&2
	exit 1
fi

%files
%license LICENSE.md
%doc README.md
/usr/bin/atomrootfsinit

%changelog
* Thu Mar 05 2026 CI Build <ci@example.com> - %{version}-1
- Automated build: added changelog entry for reproducible build systems
