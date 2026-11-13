TASK ?= task
.DEFAULT_GOAL := dev

.PHONY: install proto dev test build ci

install proto dev test build ci:
	$(TASK) $@
