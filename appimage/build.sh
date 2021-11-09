#!/bin/sh

# Create XVI AppImage.
#
# SPDX-License-Identifier: MIT
# Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

set -eu

THIS_DIR=$(cd "$(dirname "$0")" && pwd)
VERSION=$(git describe --always | sed 's/-g.*//;s/^v//;s/-/./')
TMP_DIR="/tmp/xvi_appimage"
APP_DIR="${TMP_DIR}/prefix"
APP_IMG="${TMP_DIR}/linuxdeploy-x86_64.AppImage"

cd "${THIS_DIR}/.."

# build the project
cargo build --release
install -D -m 755 "${THIS_DIR}/../target/release/xvi" "${APP_DIR}/usr/bin/xvi"

# download appimage builder
if [ ! -e "${APP_IMG}" ]; then
  wget \
    https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage \
    -O "${APP_IMG}"
  chmod 0755 "${APP_IMG}"
fi

# create appimage
cd "${TMP_DIR}"
"${APP_IMG}" --appdir "${APP_DIR}" \
             --desktop-file "${THIS_DIR}/xvi.desktop" \
             --icon-file "${THIS_DIR}/xvi.png" \
             --output appimage
mv "xvi-x86_64.AppImage" "${THIS_DIR}/../xvi-${VERSION}-x86_64.AppImage"

echo "File xvi-${VERSION}-x86_64.AppImage created"