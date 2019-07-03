`default_nettype none

module gpsdo (
	input clk_in,

  input sig_clk,
  input ref_clk,

	output ser_tx,
	input ser_rx,

	output ledr_n,
	output ledg_n,
	output ledb_n,

	output gpio_0,
	output gpio_1,
	output gpio_2,
	output gpio_3,

	output flash_csb,
	output flash_clk,
	inout  flash_io0,
	inout  flash_io1,
	// inout  flash_io2,
	// inout  flash_io3
);

  localparam SIG_BITS = 32, SYS_BITS = 32, EPOCH_BITS = 2;

	wire sig_clk_buf;
	wire ref_clk_buf;

	SB_GB gb_sig_clk (
		.USER_SIGNAL_TO_GLOBAL_BUFFER(sig_clk),
		.GLOBAL_BUFFER_OUTPUT(sig_clk_buf)
	);

	SB_GB gb_ref_clk (
		.USER_SIGNAL_TO_GLOBAL_BUFFER(ref_clk),
		.GLOBAL_BUFFER_OUTPUT(ref_clk_buf)
	);

  wire clk, clk_picosoc;
	wire locked;
	reg resetn = 0;
  pll pll(.clock_in(clk_in), .clock_out(clk), .clock_src_out(clk_picosoc), .locked(locked));

  reg [SYS_BITS-1:0] fc_sig_sys_cnt;
  reg [SIG_BITS-1:0] fc_sig_cnt;
  reg [SYS_BITS-1:0] fc_ref_sys_cnt;
  reg fc_ready;
  reg [EPOCH_BITS-1:0] fc_epoch = 0;

	FrequencyCounter cnt(
    .ref_clk(ref_clk_buf),
    .sig_clk(sig_clk_buf),
    .sys_clk(clk),
    .sig_sys_cnt(fc_sig_sys_cnt),
    .sig_cnt(fc_sig_cnt),
    .ref_sys_cnt(fc_ref_sys_cnt),
    .ready(fc_ready)
  );

  always @(posedge clk) begin
    if (fc_ready) begin
      fc_epoch <= fc_epoch + 1;
    end
  end

	wire flash_io0_oe, flash_io0_do, flash_io0_di;
	wire flash_io1_oe, flash_io1_do, flash_io1_di;
	// wire flash_io2_oe, flash_io2_do, flash_io2_di;
	// wire flash_io3_oe, flash_io3_do, flash_io3_di;

	SB_IO #(
		.PIN_TYPE(6'b 1010_01),
		.PULLUP(1'b 0)
	) flash_io_buf [1:0] (
		.PACKAGE_PIN({
			// flash_io3, flash_io2,
			flash_io1, flash_io0
		}),
		.OUTPUT_ENABLE({
			//flash_io3_oe, flash_io2_oe,
			flash_io1_oe, flash_io0_oe
		}),
		.D_OUT_0({
			//flash_io3_do, flash_io2_do,
			flash_io1_do, flash_io0_do
		}),
		.D_IN_0({
			//flash_io3_di, flash_io2_di,
			flash_io1_di, flash_io0_di
		})
	);

	wire        iomem_valid;
	reg         iomem_ready;
	wire [3:0]  iomem_wstrb;
	wire [31:0] iomem_addr;
	wire [31:0] iomem_wdata;
	reg  [31:0] iomem_rdata;

	reg  [3:0] gpio;

  generate
	always @(posedge clk_picosoc) begin
		if (!resetn) begin
			gpio <= 0;
		end else begin
			iomem_ready <= 0;
			if (iomem_valid && !iomem_ready && iomem_addr[31:24] == 8'h 03 && iomem_addr[7:0] == 8'h 00) begin
				iomem_ready <= 1;
				iomem_rdata[31:4] <= 0;
				iomem_rdata[3:0] <= gpio;
				if (iomem_wstrb[0]) gpio[3:0] <= iomem_wdata[3:0];
			end else if (iomem_valid && !iomem_ready && iomem_addr[31:24] == 8'h 03 && iomem_addr[7:0] == 8'h04) begin
				iomem_ready <= 1;
        if (SYS_BITS < 32) begin
  				iomem_rdata[31:SYS_BITS] <= 0;
        end
				iomem_rdata[SYS_BITS-1:0] <= fc_ref_sys_cnt;
			end else if (iomem_valid && !iomem_ready && iomem_addr[31:24] == 8'h 03 && iomem_addr[7:0] == 8'h08) begin
				iomem_ready <= 1;
        if (SIG_BITS < 32) begin
  				iomem_rdata[31:SIG_BITS] <= 0;
        end
				iomem_rdata[SIG_BITS-1:0] <= fc_sig_cnt;
			end else if (iomem_valid && !iomem_ready && iomem_addr[31:24] == 8'h 03 && iomem_addr[7:0] == 8'h0c) begin
				iomem_ready <= 1;
        if (SYS_BITS < 32) begin
  				iomem_rdata[31:SYS_BITS] <= 0;
        end
				iomem_rdata[SYS_BITS-1:0] <= fc_sig_sys_cnt;
			end else if (iomem_valid && !iomem_ready && iomem_addr[31:24] == 8'h 03 && iomem_addr[7:0] == 8'h10) begin
				iomem_ready <= 1;
        iomem_rdata[31:EPOCH_BITS] <= 0;
				iomem_rdata[EPOCH_BITS-1:0] <= fc_epoch;
			end
		end
	end
  endgenerate

	picosoc #(
		.BARREL_SHIFTER(0),
		.ENABLE_MULDIV(1),
		.ENABLE_COMPRESSED(1),
		.ENABLE_IRQ_QREGS(0),
		.ENABLE_COUNTERS(0),
		.MEM_WORDS(`RAM_SIZE)
	) soc (
		.clk          (clk_picosoc ),
		.resetn       (resetn      ),

		.ser_tx       (ser_tx      ),
		.ser_rx       (ser_rx      ),

		.flash_csb    (flash_csb   ),
		.flash_clk    (flash_clk   ),

		.flash_io0_oe (flash_io0_oe),
		.flash_io1_oe (flash_io1_oe),
		// .flash_io2_oe (flash_io2_oe),
		// .flash_io3_oe (flash_io3_oe),

		.flash_io0_do (flash_io0_do),
		.flash_io1_do (flash_io1_do),
		// .flash_io2_do (flash_io2_do),
		// .flash_io3_do (flash_io3_do),

		.flash_io0_di (flash_io0_di),
		.flash_io1_di (flash_io1_di),
		.flash_io2_di (1'b0),
		.flash_io3_di (1'b0),

		.irq_5        (1'b0        ),
		.irq_6        (1'b0        ),
		.irq_7        (1'b0        ),

		.iomem_valid  (iomem_valid ),
		.iomem_ready  (iomem_ready ),
		.iomem_wstrb  (iomem_wstrb ),
		.iomem_addr   (iomem_addr  ),
		.iomem_wdata  (iomem_wdata ),
		.iomem_rdata  (iomem_rdata )
	);

  reg ledg = 0, ledr = 0, ledb = 0;
  assign ledb_n = ledb;
  assign ledg_n = ledg;
  assign ledr_n = ledr;

	reg [5:0] led_pwm = 0;
	always @(posedge clk_picosoc) begin
		led_pwm <= led_pwm + 1;
  	ledg <= ref_clk & (led_pwm == 0);
  	ledr <= ~resetn;
		if (locked) begin
		  resetn <= 1;
		end
	end

	assign gpio_0 = gpio[0];
	assign gpio_1 = gpio[1];
	assign gpio_2 = gpio[2];
	assign gpio_3 = gpio[3];
endmodule
