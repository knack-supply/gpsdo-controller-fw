`default_nettype none

module FrequencyCounter(
  input ref_clk,
  input sig_clk,
  input sys_clk,
  output reg [31:0] sig_sys_cnt,
  output reg [31:0] sig_cnt,
  output reg [31:0] ref_sys_cnt,
  output reg ready = 0,
);

  wire ref_clk_buf, sig_clk_buf;
  reg state;

  pos_edge_det ref_edge_det (.sig(ref_clk), .clk(sys_clk), .pos_edge(ref_clk_buf));
  pos_edge_det sig_edge_det (.sig(sig_clk), .clk(sys_clk), .pos_edge(sig_clk_buf));

  always @(posedge sys_clk) begin
    if (ref_clk_buf) begin
      state <= 0;
    end else if (sig_clk_buf) begin
      state <= 1;
    end
  end

  wire sample_signal;
  pos_edge_det state_edge_det (.sig(state), .clk(sys_clk), .pos_edge(sample_signal));

  wire [31:0] sig_sys_cnt_unbuf, ref_sys_cnt_unbuf;
  wire [31:0] sig_cnt_unbuf;

  lfsr32 sig_sys_counter(
    .clk(sys_clk),
    .sig(1'b1),
    .cutoff(sample_signal),
    .count(sig_sys_cnt_unbuf)
  );

  lfsr32 sig_counter(
    .clk(sys_clk),
    .sig(sig_clk_buf),
    .cutoff(sample_signal),
    .count(sig_cnt_unbuf)
  );

  lfsr32 ref_sys_counter(
    .clk(sys_clk),
    .sig(1'b1),
    .cutoff(ref_clk_buf),
    .count(ref_sys_cnt_unbuf)
  );

  reg sample_signal_buf = 0, ref_clk_buf_buf = 0;

  always @(posedge sys_clk) begin
    if (sample_signal && ~sample_signal_buf) begin
      sig_sys_cnt <= sig_sys_cnt_unbuf;
      sig_cnt <= sig_cnt_unbuf;
      sample_signal_buf <= 1;
    end
    if (ref_clk_buf && ~ref_clk_buf_buf) begin
      ref_sys_cnt <= ref_sys_cnt_unbuf;
      ref_clk_buf_buf <= 1;
    end

    if (sample_signal_buf && ref_clk_buf_buf) begin
      ready <= 1;
      sample_signal_buf <= 0;
      ref_clk_buf_buf <= 0;
    end
    if (ready) begin
      ready <= 0;
    end

  end

endmodule
