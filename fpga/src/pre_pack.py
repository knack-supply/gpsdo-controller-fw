import os

DEVICE = os.environ["DEVICE"]

with open(f'src/{DEVICE}_pll_freq') as f:
    clk_freq = int(f.readlines()[0].strip())

ctx.addClock("clk_picosoc", 12)
ctx.addClock("clk", clk_freq)
# ctx.addClock("pll.clock_wire", clk_freq)

# pretend it's 50MHz for tighter packing
ctx.addClock("sig_clk", 50)
ctx.addClock("sig_clk_buf", 50)

