# SPDX-License-Identifier: MIT
# Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

THIS_FILE := $(lastword $(MAKEFILE_LIST))
THIS_DIR := $(abspath $(dir $(THIS_FILE)))

# default installation prefix
PREFIX := /usr
# version of the application
VERSION := $(shell git describe --tags --long --always | sed 's/-g.*//;s/^v//;s/-/./')
# path to extra files (mans, icon, desktop, etc)
EXTRA_DIR := $(THIS_DIR)/extra
# path to the intermediate dir
TARGET_DIR := $(THIS_DIR)/target
# path to the application binary
TARGET_BIN := $(TARGET_DIR)/release/xvi
# app image
APPIMG_DIR := $(TARGET_DIR)/appimg
APPIMG_TOOL := $(TARGET_DIR)/linuxdeploy-x86_64.AppImage
APPIMG_BIN := $(THIS_DIR)/xvi-$(VERSION)-x86_64.AppImage

all: $(TARGET_BIN)

$(TARGET_BIN):
	cargo build --release

clean:
	rm -rf $(TARGET_DIR) $(APPIMG_BIN)

version:
	@echo "$(VERSION)"

install: $(TARGET_BIN)
	install -D -m 755 $(TARGET_BIN) $(PREFIX)/bin/$(notdir $(TARGET_BIN))
	install -D -m 644 $(EXTRA_DIR)/xvi.1 $(PREFIX)/share/man/man1/xvi.1
	install -D -m 644 $(EXTRA_DIR)/xvirc.5 $(PREFIX)/share/man/man5/xvirc.5

uninstall:
	rm -f "$(PREFIX)/bin/$(notdir $(TARGET_BIN))"
	rm -f "$(PREFIX)/share/man/man1/xvi.1"
	rm -f "$(PREFIX)/share/man/man5/xvirc.5"

appimage: $(APPIMG_BIN)

$(APPIMG_BIN): $(APPIMG_TOOL)
	rm -rf $(APPIMG_DIR)/*
	$(MAKE) PREFIX=$(APPIMG_DIR)/$(PREFIX) install
	install -D -m 644 $(EXTRA_DIR)/xvi.appdata.xml $(APPIMG_DIR)/$(PREFIX)/share/metainfo/xvi.appdata.xml
	$(APPIMG_TOOL) --appdir $(APPIMG_DIR) \
		--desktop-file $(EXTRA_DIR)/xvi.desktop \
		--icon-file $(EXTRA_DIR)/xvi.png \
		--output appimage
	mv xvi-*-x86_64.AppImage $@

$(APPIMG_TOOL):
	mkdir -p $(@D)
	wget \
		https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage \
		-O $@
	chmod 0755 $@

.PHONY: all clean version install uninstall appimage
