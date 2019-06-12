module ice40_ram_2p(
  input clk,
  input wen,
  input [5:0] raddr1,
  input [5:0] raddr2,
  input [5:0] waddr,
  input [15:0] wdata,
  output reg [15:0] rdata1,
  output reg [15:0] rdata2,
);
  reg [15:0] mem [0:255];
  initial mem[0] = 0;
  always @(posedge clk) begin
    if (wen) mem[waddr] <= wdata;
    rdata1 <= mem[raddr1];
    rdata2 <= mem[raddr2];
  end
endmodule

module ice40_ram_regs (
	input clk, wen,
	input [5:0] waddr,
	input [5:0] raddr1,
	input [5:0] raddr2,
	input [31:0] wdata,
	output [31:0] rdata1,
	output [31:0] rdata2
);

  ice40_ram_2p ram0 (.clk(clk), .wen(wen), .raddr1(raddr1), .raddr2(raddr2), .waddr(waddr), .wdata(wdata[15:0]), .rdata1(rdata1[15:0]), .rdata2(rdata2[15:0]));
  ice40_ram_2p ram1 (.clk(clk), .wen(wen), .raddr1(raddr1), .raddr2(raddr2), .waddr(waddr), .wdata(wdata[31:16]), .rdata1(rdata1[31:16]), .rdata2(rdata2[31:16]));

endmodule
