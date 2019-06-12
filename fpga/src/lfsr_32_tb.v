`define assert(condition) if(!(condition)) begin $display("assertion failed at %s:%d", `__FILE__, `__LINE__); $finish(1); end

module LFSR32TB();

  reg sys_clk;
  reg periodic_clk;
  reg sig = 0;

  reg [31:0] sys_count;

  lfsr32 cnt(.clk(sys_clk), .sig(sig), .cutoff(periodic_clk), .count(sys_count));

  initial forever #1 sys_clk = ~sys_clk;

  initial begin
    sys_clk = 0;
    periodic_clk = 1;
    #2 periodic_clk = 0;
    #1 `assert(sys_count == 32'b00000000000000000000000000000001);
    sig = 1;
    #2 `assert(sys_count == 32'b10100011000000000000000000000000);
    #2 `assert(sys_count == 32'b01010001100000000000000000000000);
    #2 `assert(sys_count == 32'b00101000110000000000000000000000);
    #2 `assert(sys_count == 32'b00010100011000000000000000000000);
    #2 `assert(sys_count == 32'b00001010001100000000000000000000);
    #2 `assert(sys_count == 32'b00000101000110000000000000000000);
    #2 `assert(sys_count == 32'b00000010100011000000000000000000);
    #2 `assert(sys_count == 32'b00000001010001100000000000000000);
    #2 `assert(sys_count == 32'b00000000101000110000000000000000);
    #2 `assert(sys_count == 32'b00000000010100011000000000000000);
    #2 `assert(sys_count == 32'b00000000001010001100000000000000);
    #2 `assert(sys_count == 32'b00000000000101000110000000000000);
    #2 `assert(sys_count == 32'b00000000000010100011000000000000);
    #2 `assert(sys_count == 32'b00000000000001010001100000000000);
    #2 `assert(sys_count == 32'b00000000000000101000110000000000);
    #2 `assert(sys_count == 32'b00000000000000010100011000000000);
    #2 `assert(sys_count == 32'b00000000000000001010001100000000);
    #2 `assert(sys_count == 32'b00000000000000000101000110000000);
    #2 `assert(sys_count == 32'b00000000000000000010100011000000);
    #2 `assert(sys_count == 32'b00000000000000000001010001100000);
    #2 `assert(sys_count == 32'b00000000000000000000101000110000);
    #2 `assert(sys_count == 32'b00000000000000000000010100011000);
    #2 `assert(sys_count == 32'b00000000000000000000001010001100);
    #2 `assert(sys_count == 32'b00000000000000000000000101000110);
    #2 `assert(sys_count == 32'b00000000000000000000000010100011);
    #2 `assert(sys_count == 32'b10100011000000000000000001010001);
    #2 `assert(sys_count == 32'b11110010100000000000000000101000);
    #2 `assert(sys_count == 32'b01111001010000000000000000010100);
    #2 `assert(sys_count == 32'b00111100101000000000000000001010);
    #2 `assert(sys_count == 32'b00011110010100000000000000000101);
    #2 `assert(sys_count == 32'b10101100001010000000000000000010);

    #5
    $finish;
  end


  initial begin
    $dumpfile("dump.vcd");
    $dumpvars;
  end

endmodule
