PREFIX ?= /usr/local
DESTDIR ?=

.PHONY: all check install uninstall
all:
	cargo build --release --locked
check:
	cargo fmt --check
	cargo test --locked
	cargo clippy --locked --all-targets -- -D warnings
install:
	install -Dm755 target/release/labwc-tweaks-gtk4 $(DESTDIR)$(PREFIX)/bin/labwc-tweaks-gtk4
	install -Dm644 data/org.labwc.TweaksGtk4.desktop $(DESTDIR)$(PREFIX)/share/applications/org.labwc.TweaksGtk4.desktop
	install -Dm644 data/org.labwc.TweaksGtk4.svg $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/org.labwc.TweaksGtk4.svg
	install -Dm644 LICENSE $(DESTDIR)$(PREFIX)/share/licenses/labwc-tweaks-gtk4/LICENSE
uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/labwc-tweaks-gtk4 \
	      $(DESTDIR)$(PREFIX)/share/applications/org.labwc.TweaksGtk4.desktop \
	      $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/org.labwc.TweaksGtk4.svg \
	      $(DESTDIR)$(PREFIX)/share/licenses/labwc-tweaks-gtk4/LICENSE
