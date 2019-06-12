`default_nettype none

module pos_edge_det(
  input sig,
  input clk,
  output reg pos_edge,
  output reg neg_edge,
);

  reg [1:3] sig_dly;

  always @(posedge clk) begin
    pos_edge <= sig_dly[2] & !sig_dly[3];
    neg_edge <= sig_dly[3] & !sig_dly[2];
    sig_dly <= {sig, sig_dly[1:2]};
  end

endmodule
