#!/bin/sh

# Create XVI AppImage.
#
# SPDX-License-Identifier: MIT
# Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

set -eu

THIS_DIR=$(cd "$(dirname "$0")" && pwd)

if [ $# -ne 0 ]; then
  VERSION=$1
else
  VERSION=$(git describe --tags --long --always | sed 's/-g.*//;s/^v//;s/-/./')
fi

TMP_DIR="/tmp/xvi_appimage"
APP_DIR="${TMP_DIR}/prefix"
APP_IMG="${TMP_DIR}/linuxdeploy-x86_64.AppImage"

cd "${THIS_DIR}/.."

# build the project
cargo build --release
install -D -m 755 "${THIS_DIR}/../target/release/xvi" "${APP_DIR}/usr/bin/xvi"
install -D -m 644 "${THIS_DIR}/xvi.appdata.xml" "${APP_DIR}/usr/share/metainfo/xvi.appdata.xml"

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
