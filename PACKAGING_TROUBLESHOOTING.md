# ludusavi Packaging Troubleshooting

Common issues when building or running ludusavi packages.

## AppImage Issues

### AppImage won't run (FUSE missing)
```bash
# Ubuntu/Debian
sudo apt install fuse libfuse2

# Fedora/RHEL
sudo dnf install fuse

# Arch/Manjaro
sudo pacman -S fuse2
```

### Permission denied
```bash
chmod +x ludusavi-x86_64.AppImage
```

### AppImage runs but ludusavi doesn't launch
- Check `~/.config/ludusavi/` for logs
- Ensure GTK3/WebKit dependencies are installed
- Run with `--verbose` for debug output

## Debian/Ubuntu Package Issues

### dpkg dependency errors
```bash
sudo dpkg -i ludusavi_*.deb
sudo apt-get install -f  # Fixes missing dependencies
```

### Missing GTK/WebKit dependencies
```bash
sudo apt install libgtk-3-0 libwebkit2gtk-4.1-0 libssl3 librsvg2-2 libsqlite3-0
```

### Python module not found
Not applicable - ludusavi is a single Rust binary.

## RPM Issues (Fedora/RHEL/openSUSE)

### alien conversion fails
```bash
# Ensure you have the .deb first
wget https://github.com/TheRealFame/ludusavi-packaging/releases/download/v0.31.0/ludusavi_0.31.0_amd64.deb
alien --to-rpm --scripts ludusavi_0.31.0_amd64.deb
```

### RPM install fails with dependencies
```bash
# Install with dnf/yum to auto-resolve
sudo dnf install ./ludusavi-*.rpm
# or
sudo yum localinstall ./ludusavi-*.rpm
```

## Pacman Issues (Arch/Manjaro)

### makepkg fails
```bash
# Ensure base-devel is installed
sudo pacman -S base-devel

# Build in clean directory
makepkg -f --clean --skippgpcheck
```

### Missing dependencies at runtime
```bash
# PKGBUILD depends array should cover it, but verify:
sudo pacman -S gcc-libs glibc gtk3 webkit2gtk sqlite openssl librsvg pango cairo
```

## Common Runtime Issues

### ludusavi says "No games found"
- Check `~/.config/ludusavi/` for manifest issues
- Run with `--scan` to force re-scan
- Check that game paths are correct

### Backup/restore fails
- Ensure you have write permissions to backup directory
- Check disk space
- Verify game files aren't in use

### GUI doesn't appear
```bash
# Ensure DISPLAY/WAYLAND_DISPLAY is set
# For Wayland:
export GDK_BACKEND=wayland

# For X11:
export GDK_BACKEND=x11
```

## Build Issues (CI/CD)

### GitHub Actions: "No space left on device"
- Self-hosted runners need disk space
- Use `actions/cache` for `~/.cargo/` between runs (optional)

### Cargo: "linker `cc` not found"
```yaml
# In workflow:
- run: sudo apt-get install -y build-essential
```

### Cargo: "linker `cc` failed with exit code 1"
```yaml
# Missing system libraries:
- run: sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev libssl-dev pkg-config librsvg2-dev libsqlite3-dev
```

### AppImage: "AppStream metadata missing"
Warning only. Add `usr/share/metainfo/ludusavi.appdata.xml` to silence.

### RPM build: "alien: command not found"
```yaml
# In workflow:
- run: sudo apt-get install -y alien rpm
```

### Pacman build: "makepkg as root not allowed"
```yaml
# Use non-root user or:
makepkg --asroot  # (not recommended, but works in CI)
```

## Distribution-Specific Notes

### Ubuntu 22.04
- `libwebkit2gtk-4.1-0` available in universe
- `libssl3` available

### Ubuntu 24.04+
- All dependencies in main/universe
- `libssl3` default

### Debian 12 (Bookworm)
- `libwebkit2gtk-4.1-0` in main
- May need `libssl3` from backports

### Fedora 39/40
- Works with `dnf install ./ludusavi.rpm`
- Uses `libssl3`

### Arch Linux
- Rolling, always latest deps
- `makepkg` builds clean package

### openSUSE Tumbleweed/Leap
- Use `alien --to-rpm` then `zypper in ./ludusavi.rpm`

## Reporting Issues

If you encounter a packaging issue:
1. Check this file first
2. Run with verbose output: `./ludusavi --verbose`
3. Check `~/.config/ludusavi/logs/`
4. Open issue at https://github.com/TheRealFame/ludusavi-packaging/issues

Include:
- Distro & version
- Package format used
- Error message
- `ldd usr/bin/ludusavi` output (for missing libs)