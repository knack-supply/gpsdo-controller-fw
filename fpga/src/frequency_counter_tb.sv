`timescale 1us/1ns

module FrequencyCounterTB();

  reg ref_clk;
  reg sig_clk;
  reg sys_clk;

  reg int sig_clk_cnt = 0, sys_clk_cnt = 0, ref_clk_cnt = 0;

  reg [31:0] sig_sys_cnt;
  reg [31:0] sig_cnt;
  reg [31:0] ref_sys_cnt;
  reg ready;

  reg int measure_counts = 0; // 0 - PRE, 1 - MEASURE, 2 - FINISH, -1 - DONE

  FrequencyCounter cnt(
    .ref_clk(ref_clk),
    .sig_clk(sig_clk),
    .sys_clk(sys_clk),
    .sig_sys_cnt(sig_sys_cnt),
    .sig_cnt(sig_cnt),
    .ref_sys_cnt(ref_sys_cnt),
    .ready(ready)
  );

  initial forever #23712 ref_clk = ~ref_clk;
  initial forever #6.771 sig_clk = ~sig_clk;
  initial forever #1.120 sys_clk = ~sys_clk;

  initial begin
    ref_clk = 0;
    sig_clk = 0;
    sys_clk = 0;

    #300000
    measure_counts = 2;
    #100000

    $finish;
  end

  always @(posedge sig_clk) begin
    if (measure_counts > 0) begin
      sig_clk_cnt <= sig_clk_cnt + 1;
    end
  end

  always @(posedge sys_clk) begin
    if (measure_counts > 0) begin
      sys_clk_cnt <= sys_clk_cnt + 1;
    end
  end

  always @(posedge ref_clk) begin
    if (measure_counts == 2) begin
      measure_counts <= -1;
    end else begin
      if (ref_clk_cnt == 0) begin
        measure_counts <= 1;
      end
      if (measure_counts >= 0) begin
	      ref_clk_cnt <= ref_clk_cnt + 1;
      end
    end
  end

  always @(posedge ready) begin
    $display("counters:\t%f\t%f\t%f", ref_sys_cnt, sig_cnt, sig_sys_cnt);
  end

  initial begin
    $dumpfile("dump.vcd");
    $dumpvars;
  end

endmodule
