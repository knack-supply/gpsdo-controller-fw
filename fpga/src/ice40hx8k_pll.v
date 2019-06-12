`default_nettype none

module pll(
	input  clock_in,
	output clock_out,
	output clock_src_out,
	output locked,
);

  wire clock_out_unbuf;

  SB_PLL40_CORE #(
    `include "target/ice40hx8k_pll_params.v"
  ) uut (
    .LOCK(locked),
    .RESETB(1'b1),
    .BYPASS(1'b0),
    .REFERENCECLK(clock_in),
    .PLLOUTGLOBAL(clock_out_unbuf),
  );

	SB_GB gb_clk (
		.USER_SIGNAL_TO_GLOBAL_BUFFER(clock_out_unbuf),
		.GLOBAL_BUFFER_OUTPUT(clock_out)
	);

	SB_GB gb_src_clk (
		.USER_SIGNAL_TO_GLOBAL_BUFFER(clock_in),
		.GLOBAL_BUFFER_OUTPUT(clock_src_out)
	);

endmodule
