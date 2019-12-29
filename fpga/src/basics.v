`default_nettype none

module sr_ff(
  input s,
  input r,
  input clk,
  output reg q = 0,
  output reg qn = 0
);

  always @(posedge clk) begin
    if (s != r) begin
      q <= s;
      qn <= r;
    end else if (s && r) begin
      q <= 1;
      qn <= 1;
    end
  end

endmodule

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

module cdc_pulse(
  input in_clk,
  input in_pulse,
  input out_clk,
  output reg out_pulse,
);

  wire busy;
  reg req, last_req, new_req, xreq_pipe;

  always @(posedge in_clk)
    if (!busy && in_pulse)
      req <= 1'b1;
    else if (out_pulse)
      req <= 1'b0;
  assign busy = req || out_pulse;

  always @(posedge out_clk)
    { last_req, new_req, xreq_pipe } <= { new_req, xreq_pipe, req };

  always @(posedge out_clk)
    out_pulse <= !last_req && new_req;

endmodule
