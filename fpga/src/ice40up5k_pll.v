`default_nettype none

module pll(
	input  clock_in,
	output clock_out,
	output clock_src_out,
	output locked,
);

  SB_PLL40_2_PAD #(
    `include "target/ice40up5k_pll_params.v"
  ) uut (
    .LOCK(locked),
    .RESETB(1'b1),
    .BYPASS(1'b0),
    .PACKAGEPIN(clock_in),
    .PLLOUTGLOBALB(clock_out),
    .PLLOUTGLOBALA(clock_src_out),
  );

endmodule
