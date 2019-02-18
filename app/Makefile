TARGET=thumbv7em-none-eabi
APP=app

CARGO ?= cargo

RUSTFLAGS_FOR_CARGO_LINKING ?= -C link-arg=-Tapp.ld \
-C linker=rust-lld \
-C relocation-model=static

# TODO: Add this later
# -D warnings

# http://stackoverflow.com/questions/10858261/abort-makefile-if-variable-not-set
# Check that given variables are set and all have non-empty values,
# die with an error otherwise.
#
# Params:
#   1. Variable name(s) to test.
#   2. (optional) Error message to print.
check_defined = \
    $(strip $(foreach 1,$1, \
        $(call __check_defined,$1,$(strip $(value 2)))))
__check_defined = \
    $(if $(value $1),, \
      $(error Undefined $1$(if $2, ($2))))


$(call check_defined, APP)
$(call check_defined, TARGET)

# If environment variable V is non-empty, be verbose
ifneq ($(V),)
Q=
VERBOSE = --verbose
else
Q=@
VERBOSE =
endif

SIZE      ?= arm-none-eabi-size

# Dump configuration for verbose builds
ifneq ($(V),)
  $(info )
  $(info **************************************************)
  $(info TOCK RUST STATIC APP BUILD SYSTEM -- VERBOSE BUILD)
  $(info **************************************************)
  $(info Config:)
  $(info APP=$(APP))
  $(info TARGET=$(TARGET))
  $(info )
  $(info rustc --version = $(shell rustc --version))
  $(info **************************************************)
  $(info )
endif

.PRECIOUS: %.elf
# Support rules

# User-facing targets
.PHONY: all
all: release

.PHONY: clean
clean:
	$(Q)$(CARGO) clean $(VERBOSE)

.PHONY: release
release: target/$(TARGET)/release/$(APP).elf

.PHONY: debug
debug: target/$(TARGET)/debug/$(APP).elf

.PHONY: release
release: target/$(TARGET)/release/$(APP).elf


# Support rules

target:
	@mkdir -p target

# Cargo outputs an elf file (just without a file extension)
%.elf: %
	$(Q)cp $< $@

# Cargo-drivers
# We want to always invoke cargo (yay nested build systems), so these need to
# be phony, which means they can't be pattern rules.

APP_FILE_NAME="$(APP)"
ELF_FILE_NAME="target/tab/$(APP_FILE_NAME)/cortex-m4.elf"
TAB_FILE_NAME="target/tab/$(APP_FILE_NAME).tab"

.PHONY: target/$(TARGET)/release/$(APP)
target/$(TARGET)/release/$(APP):
	$(Q)RUSTFLAGS="$(RUSTFLAGS_FOR_CARGO_LINKING)" $(CARGO) build --target=$(TARGET) $(VERBOSE) --release
	$(Q)$(SIZE) $@

.PHONY: target/$(TARGET)/debug/$(APP)
target/$(TARGET)/debug/$(APP):
	$(Q)RUSTFLAGS="$(RUSTFLAGS_FOR_CARGO_LINKING)" $(CARGO) build $(VERBOSE) --target=$(TARGET)
	$(Q)$(SIZE) $@
	mkdir -p "target/tab/$(APP_FILE_NAME)"
	cp "target/$(TARGET)/debug/$(APP)" $(ELF_FILE_NAME)
	# NOTE: `--stack 2048` *cannot* be changed unless a corresponding
	#       change it made to `STACK_SIZE` in `app.ld`
	elf2tab -n $(APP_FILE_NAME) -v -o $(TAB_FILE_NAME) $(ELF_FILE_NAME) --stack 2048 --app-heap 4096 --kernel-heap 1024 --protected-region-size=64
