#!/usr/bin/env bash
set -euo pipefail

echo "==> Instalando dependencias de desarrollo de Fedora para Floral Notepaper..."
sudo dnf install -y \
  webkit2gtk4.1-devel \
  gtk3-devel \
  libsoup3-devel \
  librsvg2-devel \
  libappindicator-gtk3-devel \
  dbus-devel \
  openssl-devel \
  rpm-build \
  patchelf

echo "==> Sincronizando enlaces locales de compilación..."
node "$(dirname "$0")/ensure-linux-deps.mjs"

echo "==> Listo. Puedes ejecutar 'npm run tauri dev' o 'npm run tauri -- build --bundles rpm'."
