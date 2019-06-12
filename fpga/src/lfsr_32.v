`default_nettype none

module lfsr32(
  input clk,
  input sig,
  input cutoff,
  output reg [31:0] count = 0,
  output wire ready
);

  localparam seed0 = 32'b00000000000000000000000000000001; // 1
  localparam seed1 = 32'b10100011000000000000000000000000; // 2734686208

  assign ready = cutoff;

  always @(posedge clk) begin
    if (cutoff) begin
      count <= sig ? seed1 : seed0;
    end else if (sig) begin
      // 32, 30, 26, 25 (1 based)
      // 31, 29, 25, 24 (0 based)
      count[31] <= count[0]; // 0 -> 31
      count[30] <= count[31];
      count[29] <= count[30] ^ count[0]; // 30 -> 29
      count[28:26] <= count[29:27];
      count[25] <= count[26] ^ count[0]; // 26 -> 25
      count[24] <= count[25] ^ count[0]; // 25 -> 24
      count[23:0] <= count[24:1];
    end
  end

endmodule
