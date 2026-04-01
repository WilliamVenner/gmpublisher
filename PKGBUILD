# Maintainer: Lythium max@lythium.dev

pkgname=gmpublisher-bin
_realname=gmpublisher
pkgver=2.11.3
pkgrel=1
pkgdesc="Workshop Publishing Utility for Garry's Mod, written in Rust & Svelte and powered by Tauri"
arch=('x86_64')
url="https://github.com/WilliamVenner/gmpublisher"
license=('GPL-3.0')
depends=('webkit2gtk' 'hicolor-icon-theme' 'libappindicator-gtk3' 'gst-plugins-good' 'gst-plugins-bad' 'gst-libav')
makedepends=('unzip')
provides=("${_realname}")
conflicts=("${_realname}")
source=("${_realname}_linux64.zip::https://github.com/WilliamVenner/${_realname}/releases/download/${pkgver}/${_realname}_linux64.zip"
        "LICENSE::https://raw.githubusercontent.com/WilliamVenner/${_realname}/${pkgver}/LICENSE"
        "${_realname}.png::https://raw.githubusercontent.com/WilliamVenner/${_realname}/${pkgver}/src-tauri/icons/128x128.png")
sha256sums=('3557f4117aed96652c76590f5727953868a377b1dee6391cdc0e14decd36252d'
            'd1bfc7a749950924bd5d27dee610ac8933f96badc14a774be796e986b9dcd3aa'
            '5e9fb667ca5095013921970c58b53f984f4ba0836edf6b78876e03861b5f4265')

package() {
  install -Dm755 "${srcdir}/${_realname}" "$pkgdir/usr/lib/${_realname}/${_realname}"
  install -Dm644 "${srcdir}/libsteam_api.so" "$pkgdir/usr/lib/${_realname}/libsteam_api.so"

  install -d "$pkgdir/usr/bin"
  cat << EOF > "$pkgdir/usr/bin/${_realname}"
#!/bin/sh
export LD_LIBRARY_PATH=/usr/lib/${_realname}
exec /usr/lib/${_realname}/${_realname} "\$@"
EOF

  chmod 755 "$pkgdir/usr/bin/${_realname}"

  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"

  install -Dm644 "${srcdir}/${_realname}.png" "$pkgdir/usr/share/icons/hicolor/128x128/apps/${_realname}.png"

  install -d "$pkgdir/usr/share/applications"
  cat << EOF > "$pkgdir/usr/share/applications/${_realname}.desktop"
[Desktop Entry]
Name=gmpublisher
Comment=${pkgdesc}
Exec=${_realname}
Icon=${_realname}
Type=Application
Categories=Utility;Game;
EOF
}

post_install() {
  /usr/bin/gtk-update-icon-cache -q -t applications -f /usr/share/icons/hicolor
}

post_upgrade() {
  post_install
}

post_remove() {
  /usr/bin/gtk-update-icon-cache -q -t applications -f /usr/share/icons/hicolor
}
sha256sums=('3557f4117aed96652c76590f5727953868a377b1dee6391cdc0e14decd36252d'
            'd1bfc7a749950924bd5d27dee610ac8933f96badc14a774be796e986b9dcd3aa'
            '5e9fb667ca5095013921970c58b53f984f4ba0836edf6b78876e03861b5f4265')
