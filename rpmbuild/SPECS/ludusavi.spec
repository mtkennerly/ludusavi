Name:           ludusavi
Version:        0.31.0
Release:        1%{?dist}
Summary:        Game save backup tool
License:        MIT
URL:            https://github.com/mtkennerly/ludusavi
Source0:        https://github.com/mtkennerly/ludusavi/archive/v%{version}/ludusavi-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  libwebkit2gtk3-devel
BuildRequires:  libgtk3-devel
BuildRequires:  libsqlite3-devel
BuildRequires:  librsvg2-devel
BuildRequires:  openssl-devel
BuildRequires:  pkgconfig

%description
ludusavi is a tool for backing up your PC video game save data.
It supports over 19,000 games plus custom entries, and multiple game
stores (Steam, GOG, Epic, Heroic, Lutris, etc.).
It provides both a graphical interface and command line interface.

%prep
%autosetup

%build
cargo build --release --locked

%install
cargo install --locked --root=%{buildroot}/usr --path=. --bins
rm -f %{buildroot}/usr/.crates.toml %{buildroot}/usr/.crates2.json

%files
%{_bindir}/ludusavi
%license LICENSE
%doc README.md CHANGELOG.md

%changelog
* Mon Jul 29 2024 Matthew T. Kennerly <mtkennerly@gmail.com> - 0.31.0-1
- Initial RPM package release