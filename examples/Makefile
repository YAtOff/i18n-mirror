# Demo-project targets (run from examples/).
# The extractor is either installed (pip/uv/cargo) or built from ../Cargo.toml.

PY := .venv/bin/python
LOCALES ?= bg
I18N_MIRROR ?= $(shell command -v i18n-mirror 2>/dev/null || echo ../target/release/i18n-mirror)

.PHONY: help venv gettext extract messages compile verify demo test clean

help:
	@echo "targets:"
	@echo "  venv       create .venv and install Django"
	@echo "  gettext    bootstrap gettext-tools without root (if missing)"
	@echo "  extract    mirror frontend/src into build/i18n-mirror"
	@echo "  messages   extract + makemessages for both domains  [LOCALES=bg]"
	@echo "  compile    compilemessages (po → mo)"
	@echo "  verify     end-to-end runtime check of the jsi18n catalog (node)"
	@echo "  demo       compile + run the Django demo server on :8000"
	@echo "  test       extractor tests (../) + frontend typecheck/build"

venv:
	python3 -m venv --without-pip .venv
	pip --python $$(pwd)/.venv/bin/python install django

gettext:
	scripts/bootstrap-gettext.sh

extract:
	@test -x $(I18N_MIRROR) || cargo build --release --quiet --manifest-path ../Cargo.toml
	$(I18N_MIRROR) --source frontend/src --output build/i18n-mirror \
		--functions gettext,gettext_noop,gettext_lazy,ngettext,ngettext_lazy,pgettext,pgettext_lazy,npgettext,npgettext_lazy,_,t,nt=ngettext,pt=pgettext,npt=npgettext

messages:
	scripts/extract-messages.sh $(LOCALES)

compile:
	$(PY) manage.py compilemessages -l $(LOCALES)

verify:
	node tests/verify_catalog.mjs

demo: compile
	$(PY) manage.py runserver 127.0.0.1:8000

test:
	cargo test --manifest-path ../Cargo.toml
	cd frontend && npm run build

clean:
	rm -rf build
