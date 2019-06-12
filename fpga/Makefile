.SUFFIXES:
.SECONDEXPANSION:
.SECONDARY:

targets_ice40 = ice40up5k ice40hx8k

target_ice40up5k_nextpnr_opts = --up5k --package sg48
target_ice40hx8k_nextpnr_opts = --hx8k --package ct256

target_ice40up5k_icetime_opts = -d up5k
target_ice40hx8k_icetime_opts = -d hx8k

targets = $(targets_ice40)

src = src/basics.v \
		src/picosoc.v \
		picorv32/picosoc/simpleuart.v \
		picorv32/picosoc/spimemio.v \
		picorv32/picorv32.v \
		src/frequency_counter.sv \
		src/lfsr_32.v \
		src/gpsdo.v

ice40_src = \
		src/ice40_ram_regs.v \
		$(src)

ice40up5k_src = \
		src/config_ice40up5k.v \
		src/ice40up5k_pll.v \
		picorv32/picosoc/ice40up5k_spram.v \
		$(ice40_src)

ice40hx8k_src = \
		src/config_ice40hx8k.v \
		src/ice40hx8k_pll.v \
		$(ice40_src)

included_src =

ice40up5k_included_src = \
		target/ice40up5k_pll_params.v \
		$(included_src)

ice40hx8k_included_src = \
		target/ice40hx8k_pll_params.v \
		$(included_src)

# ice40up5ksim: ice40up5k_tb.vvp ice40up5k_fw.hex
# 	vvp -N $< +firmware=ice40up5k_fw.hex

# ice40up5ksynsim: ice40up5k_syn_tb.vvp ice40up5k_fw.hex
# 	vvp -N $< +firmware=ice40up5k_fw.hex

target/ice40%.json: $$(ice40$$*_src) $$(ice40$$*_included_src)
	yosys -ql target/ice40$*.log -p 'synth_ice40 -dsp -retime -relut -top gpsdo -json $@' $(ice40$*_src)

# target/ice40up5k_tb.vvp: src/gpsdo_tb.v $(src) $(included_src)
# 	iverilog -s testbench -o $@ $(src) `yosys-config --datdir/ice40/cells_sim.v`

# target/ice40up5k_syn_tb.vvp: target/ice40up5k_syn.v src/gpsdo_tb.v $(src) $(included_src)
# 	iverilog -s testbench -o $@ $(src) `yosys-config --datdir/ice40/cells_sim.v`

# target/%_syn.v: target/$*.json
# 	yosys -p 'read_json $^; write_verilog $@'

target/ice40%_pll_params.v: src/ice40%_pll_freq
	icepll -q -o `cat $^` -f $@

target/ice40%.asc: src/pre_pack.py src/ice40%.pcf target/ice40%.json src/seed
	DEVICE=ice40$* nextpnr-ice40 --seed `cat src/seed` --opt-timing $(target_ice40$*_nextpnr_opts) --asc target/ice40$*.asc --pcf src/ice40$*.pcf --pre-pack src/pre_pack.py --json target/ice40$*.json

target/ice40%.bin: target/ice40%.asc
	icetime $(target_ice40$*_icetime_opts) -c 12 -mtr target/ice40$*.rpt $<
	icepack $< $@

$(addsuffix prog,$(targets_ice40)): %: %_fpga iceprog_fw

$(addsuffix prog_fpga,$(targets_ice40)): %prog_fpga: target/%.bin
	iceprog $<

$(addsuffix reprog_fpga,$(targets_ice40)): %reprog_fpga:
	iceprog target/$*.bin

iceprog_fw: target/fw.bin
	iceprog -o 1M $<

target/fw.bin: ../fw/target/riscv32imc-unknown-none-elf/release/ks-gpsdo
	llvm-objcopy -O binary $< $@

../fw/target/riscv32imc-unknown-none-elf/release/ks-gpsdo:
	cd ../fw && cargo build --release

clean:
	rm -f target/*

.PHONY: clean ice40up5kprog ice40up5kprog_fpga iceprog_fw ice40up5ksim ice40up5ksynsim
.PHONY: ../fw/target/riscv32imc-unknown-none-elf/release/ks-gpsdo
