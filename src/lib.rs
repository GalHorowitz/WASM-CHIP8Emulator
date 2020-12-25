mod utils;

use wasm_bindgen::prelude::*;

const MEM_SIZE: usize = 4096;
const MEM_RESERVED: usize = 512;
const SCREEN_WIDTH: usize = 64;
const SCREEN_HEIGHT: usize = 32;

#[wasm_bindgen]
/// Represents a CHIP-8 CPU
pub struct Cpu {
    // The available CPU memory. While the entire range is addressable, the first 512 bytes are
    // reserved for the interpreter. We we only use them to store the font sprites needed for
    // instruction Fx29.
    memory: [u8; MEM_SIZE],

    // Stores the call-site address, i.e. the instruction before the return address. This
    // is done so we can execute a `CALL` at the last memory address and not have to deal with
    // wrapping. The original CHIP-8 supports call/ret instructions, with up to 12 nested calls. We
    // allow up to 16.
    call_stack: [usize; 16],

    // 16 available registers named V0 through VF. VF is used as a flag in some instructions.
    v_registers: [u8; 16],

    // The I register is used to address memory in some instructions. Register size is 12 bits.
    i_register: usize,

    // The address of the next instruction to execute. Register size is 12 bits.
    pc_register: usize,

    // The index of first free `call_stack` cell.
    sp_register: usize,

    // The delay timer and the sound timer registers count down at 60HZ when not zero. If the sound
    // timer register is not zero a beep is heard.
    dt_register: u8,
    st_register: u8,

    // Internal screen buffer which is updated by draw/clear instructions. The screen is
    // monochromatic: a pixel is `true` if it is turned on.
    screen_buffer: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    // Screen buffer dirty flag. This flag is set whenever the internal buffer is changed. The
    // actual display must update and then clear this flag.
    screen_dirty: bool,

    // Current keyboard state
    key_state: [bool; 16],
    // Waiting for key press flag. This flag is set whenever the cpu is waiting for a captured key
    // press. The flag is reset by the instruction which set the flag in the first place, after
    // receiving the captured key.
    waiting_for_keypress: bool,
    // The key that was captured.
    captured_key: u8,

    // Options that change how some instructions operate. Used to emulate ROMs that depend on
    // interpreter quirks from different platforms.
    original_shift: bool, 
    original_mem_acc: bool,
}

#[wasm_bindgen]
impl Cpu {
    /// Construct a CHIP-8 cpu at the intial entry state.
    pub fn new() -> Self {
        utils::set_panic_hook();

        let mut initial_memory = [0u8; MEM_SIZE];
        // Initialize the font sprites at the start of memory.
        initial_memory[..5 * 16].copy_from_slice(&[
            0xF0, 0x90, 0x90, 0x90, 0xF0, // '0'
            0x20, 0x60, 0x20, 0x20, 0x70, // '1'
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // '2'
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // '3'
            0x90, 0x90, 0xF0, 0x10, 0x10, // '4'
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // '5'
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // '6'
            0xF0, 0x10, 0x20, 0x40, 0x40, // '7'
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // '8'
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // '9'
            0xF0, 0x90, 0xF0, 0x90, 0x90, // 'A'
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // 'B'
            0xF0, 0x80, 0x80, 0x80, 0xF0, // 'C'
            0xE0, 0x90, 0x90, 0x90, 0xE0, // 'D'
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // 'E'
            0xF0, 0x80, 0xF0, 0x80, 0x80, // 'F'
        ]);

        Cpu {
            memory: initial_memory,
            call_stack: [0; 16],
            v_registers: [0; 16],
            i_register: 0,
            pc_register: MEM_RESERVED, // Defined entry point
            sp_register: 0,
            dt_register: 0,
            st_register: 0,
            screen_buffer: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            screen_dirty: false,
            key_state: [false; 16],
            waiting_for_keypress: false,
            captured_key: 0,
            original_shift: false,
            original_mem_acc: false
        }
    }

    /// Construct a CHIP-8 cpu at the initial entry state, with rom bytes loaded at the entry point
    /// in memory.
    pub fn with_rom(rom: &[u8]) -> Self {
        assert!(rom.len() <= MEM_SIZE - MEM_RESERVED, "ROM file too large to fit in memory");

        let mut init_cpu = Cpu::new();

        // Copy rom bytes to the memory at the entry point (right after the reserved memory)
        init_cpu.memory[MEM_RESERVED..MEM_RESERVED + rom.len()].copy_from_slice(rom);

        init_cpu
    }

    /// Construct a CHIP-8 cou at the initial entry state, with rom bytes loaded at the entry point
    /// in memory.
    /// When `original_shift` is true, the original behaviour of the shift instructions is used,
    /// i.e. the instruction shifts Vy instead of Vx.
    /// When `original_mem_acc` is true, the original behaviour of the load/store instructions is
    /// used, i.e. the instructions increment the I register by the number of registers used.
    pub fn with_rom_and_options(rom: &[u8], original_shift: bool, original_mem_acc: bool) -> Self {
        let mut init_cpu = Cpu::with_rom(rom);
        init_cpu.original_shift = original_shift;
        init_cpu.original_mem_acc = original_mem_acc;

        init_cpu
    }

    /// Decode and execute one instruction.
    /// It is the responsibility of the caller to check the `waiting_for_keypress` flag. If it is
    /// set, The caller should only call `step` again after calling `set_captured_key`.
    /// It is the responsibility of the caller to check the `screen_dirty` flag and update the
    /// display if needed.
    pub fn step(&mut self) {
        assert!(self.pc_register + 1 < MEM_SIZE, "PC out of memory bounds");

        // Instructions are 2 bytes, big-endian.
        let instruction: u16 =
            ((self.memory[self.pc_register] as u16) << 8) | (self.memory[self.pc_register + 1] as u16);

        // println!("Executing instruction {:x} at address {:#x}", instruction, self.pc_register);
        
        // Decode instruction. The instruction type is determined by the most significant nibble.
        match (instruction & 0xF000) >> 12 {
            0x0 => {
                if instruction == 0x00E0 {
                    self.instr_00e0(instruction);
                } else if instruction == 0x00EE {
                    self.instr_00ee(instruction);
                } else {
                    panic!("Unknown SYS 0nnn instruction");
                }
            }
            0x1 => self.instr_1nnn(instruction),
            0x2 => self.instr_2nnn(instruction),
            0x3 => self.instr_3xkk(instruction),
            0x4 => self.instr_4xkk(instruction),
            0x5 => self.instr_5xy0(instruction),
            0x6 => self.instr_6xkk(instruction),
            0x7 => self.instr_7xkk(instruction),
            0x8 => {
                // These are arithmetic and logic operations: 8xyT, where the last nibble determines
                // the operation type. 
                match instruction & 0xF {
                    0x0 => self.instr_8xy0(instruction),
                    0x1 => self.instr_8xy1(instruction),
                    0x2 => self.instr_8xy2(instruction),
                    0x3 => self.instr_8xy3(instruction),
                    0x4 => self.instr_8xy4(instruction),
                    0x5 => self.instr_8xy5(instruction),
                    0x6 => self.instr_8xy6(instruction),
                    0x7 => self.instr_8xy7(instruction),
                    0xE => self.instr_8xye(instruction),
                    _ => panic!("Unknown 8xyT instruction type"),
                }
            }
            0x9 => self.instr_9xy0(instruction),
            0xA => self.instr_annn(instruction),
            0xB => self.instr_bnnn(instruction),
            0xC => self.instr_cxkk(instruction),
            0xD => self.instr_dxyn(instruction),
            0xE => {
                // These are keyboard flow-control instructions: ExTT, where the last byte
                // determines the instruction type.
                match instruction & 0xFF {
                    0x9E => self.instr_ex9e(instruction),
                    0xA1 => self.instr_exa1(instruction),
                    _ => panic!("Unknown ExTT instruction type")
                }
            }
            0xF => {
                // These are general peripheral devices/memory instructions: FxTT, where the last
                // byte determines the instruction type.
                match instruction & 0xFF {
                    0x07 => self.instr_fx07(instruction),
                    0x0A => self.instr_fx0a(instruction),
                    0x15 => self.instr_fx15(instruction),
                    0x18 => self.instr_fx18(instruction),
                    0x1E => self.instr_fx1e(instruction),
                    0x29 => self.instr_fx29(instruction),
                    0x33 => self.instr_fx33(instruction),
                    0x55 => self.instr_fx55(instruction),
                    0x65 => self.instr_fx65(instruction),
                    _ => panic!("Unknown FxTT instruction type"),
                }
            }
            _ => unreachable!(),
        }

        // Increment PC
        self.pc_register += 2;
    }

    /// Tick internal cpu timers. Must be called at 60HZ.
    pub fn tick_clock(&mut self) {
        if self.dt_register > 0 {
            self.dt_register -= 1;
        }

        if self.st_register > 0 {
            self.st_register -= 1;
        }
    }

    /// Get a pointer to the screen buffer memory, used from the JS side to render the screen.
    pub fn get_screen_buffer(&self) -> *const bool {
        self.screen_buffer.as_ptr()
    }

    /// Returns whether or not the screen dirty, and if it is, sets it to false.
    pub fn handle_screen_dirty_flag(&mut self) -> bool {
        let captured_flag = self.screen_dirty;
        self.screen_dirty = false;
        captured_flag
    }

    /// Update the internal key state to the provided key state.
    /// `new_key_state` must be of length 16.
    pub fn update_key_state(&mut self, new_key_state: &[u8]) {
        assert!(new_key_state.len() == 16);
        
        assert!(std::mem::size_of::<bool>() == 1);
        // `wasm_bindgen` doesn't support passing boolean arrays, so we must pass it as a byte array
        // and then convert it back.
        let new_key_state = unsafe {
            std::slice::from_raw_parts(new_key_state.as_ptr() as *const bool, 16)
        };

        &mut self.key_state[..].copy_from_slice(new_key_state.into());
    }

    /// Returns true if the cpu is waiting for a captured key press.
    pub fn is_waiting_for_keypress(&self) -> bool {
        self.waiting_for_keypress
    }

    /// Sets the key that was captured in the last keypress.
    pub fn set_captured_key(&mut self, captured_key: u8) {
        assert!(self.waiting_for_keypress,
            "Received a captured key even though we are not waiting for a key press");
        
        self.captured_key = captured_key;
    }

    /// Returns true if the emulator should play a tone
    pub fn should_play_tone(&self) -> bool {
        self.st_register > 0
    }
}

use rand::Rng;

// Instruction implementations
impl Cpu {
    /// Execute `CLS` instruction
    fn instr_00e0(&mut self, _instr: u16) {
        for pixel in self.screen_buffer.iter_mut() {
            *pixel = false;
        }

        self.screen_dirty = true;
    }

    /// Execute `RET` instruction
    fn instr_00ee(&mut self, _instr: u16) {
        assert!(self.sp_register > 0, "RET without CALL");

        // Reclaim top of stack
        self.sp_register -= 1;
        // Update pc to popped return address. The stored address is the address of the call-site,
        // which will get incremented after this instruction is executed, so the correct instruction
        // will be executed next.
        self.pc_register = self.call_stack[self.sp_register];
    }

    /// Execute `JP addr` instruction
    fn instr_1nnn(&mut self, instr: u16) {
        let jump_target = decode_instr_addr(instr);
        assert!(jump_target >= MEM_RESERVED, "Jumping to the reserved memory area is not allowed");
        // assert!(jump_target%2 == 0, "Unaligned jumps are not allowed");

        // Update pc to jump target. We subtract 2, so after the PC is incremented the jump
        // target is the next instruction that will be executed. We aren't able to jump to address
        // 0, so no wrapping is possible.
        self.pc_register = jump_target - 2;
    }

    /// Execute `CALL addr` instruction
    fn instr_2nnn(&mut self, instr: u16) {
        let call_target = decode_instr_addr(instr);
        assert!(call_target >= MEM_RESERVED, "Jumping to the reserved memory area is not allowed");
        // assert!(call_target%2 == 0, "Unaligned jumps are not allowed");

        assert!(self.sp_register < self.call_stack.len(), "Too many CALLs, recursion depth limit reached");

        // Store our current address at the top of the call stack
        self.call_stack[self.sp_register] = self.pc_register;
        // Update top of stack index
        self.sp_register += 1;

        // Update pc to jump target. We subtract 2, so after the PC is incremented the jump
        // target is the next instruction that will be executed. We aren't able to jump to address
        // 0, so no wrapping is possible.
        self.pc_register = call_target - 2;
    }

    /// Execute `SE Vx, byte` instruction
    fn instr_3xkk(&mut self, instr: u16) {
        // Skip the next instruction if the register value and the byte are equal
        if self.v_registers[decode_instr_x_reg(instr)] == decode_instr_byte_imm(instr) {
            self.pc_register += 2;
        }
    }

    /// Execute `SNE Vx, byte` instruction
    fn instr_4xkk(&mut self, instr: u16) {
        // Skip the next instruction if the register value and the byte are not equal
        if self.v_registers[decode_instr_x_reg(instr)] != decode_instr_byte_imm(instr) {
            self.pc_register += 2;
        }
    }

    /// Execute `SE Vx, Vy` instruction
    fn instr_5xy0(&mut self, instr: u16) {
        let x_register = decode_instr_x_reg(instr);
        let y_register = decode_instr_y_reg(instr);

        // Skip the next instruction if the values of the registers are equal
        if self.v_registers[x_register] == self.v_registers[y_register] {
            self.pc_register += 2;
        }
    }

    /// Execute `LD Vx, byte` instruction
    fn instr_6xkk(&mut self, instr: u16) {
        self.v_registers[decode_instr_x_reg(instr)] = decode_instr_byte_imm(instr);
    }

    /// Execute `ADD Vx, byte` instruction
    fn instr_7xkk(&mut self, instr: u16) {
        let register_idx = decode_instr_x_reg(instr);
        let byte_imm = decode_instr_byte_imm(instr);
        self.v_registers[register_idx] = self.v_registers[register_idx].wrapping_add(byte_imm);
    }

    /// Execute `LD Vx, Vy` instruction
    fn instr_8xy0(&mut self, instr: u16) {
        let x_register = decode_instr_x_reg(instr);
        let y_register = decode_instr_y_reg(instr);
        self.v_registers[x_register] = self.v_registers[y_register];
    }

    /// Execute `OR Vx, Vy` instruction
    fn instr_8xy1(&mut self, instr: u16) {
        let x_register = decode_instr_x_reg(instr);
        let y_register = decode_instr_y_reg(instr);
        self.v_registers[x_register] |= self.v_registers[y_register];
    }

    /// Execute `AND Vx, Vy` instruction
    fn instr_8xy2(&mut self, instr: u16) {
        let x_register = decode_instr_x_reg(instr);
        let y_register = decode_instr_y_reg(instr);
        self.v_registers[x_register] &= self.v_registers[y_register];
    }

    /// Execute `XOR Vx, Vy` instruction
    fn instr_8xy3(&mut self, instr: u16) {
        let x_register = decode_instr_x_reg(instr);
        let y_register = decode_instr_y_reg(instr);
        self.v_registers[x_register] ^= self.v_registers[y_register];
    }

    /// Execute `ADD Vx, Vy` instruction
    fn instr_8xy4(&mut self, instr: u16) {
        let x_register = decode_instr_x_reg(instr);
        let y_register = decode_instr_y_reg(instr);
        let (sum, carry) = self.v_registers[x_register].overflowing_add(self.v_registers[y_register]);
        
        // After performing register addition, VF acts as a carry flag
        self.v_registers[x_register] = sum;
        self.v_registers[0xF] = carry as u8;
    }

    /// Execute `SUB Vx, Vy` instruction
    fn instr_8xy5(&mut self, instr: u16) {
        let x_register = decode_instr_x_reg(instr);
        let y_register = decode_instr_y_reg(instr);
        let (diff, borrow) = self.v_registers[x_register].overflowing_sub(self.v_registers[y_register]);
        
        // After performing register subtraction, VF acts as a NOT borrow flag
        self.v_registers[x_register] = diff;
        self.v_registers[0xF] = (!borrow) as u8;
    }

    /// Execute `SHR Vx, Vy` instruction
    fn instr_8xy6(&mut self, instr: u16) {
        let x_register = decode_instr_x_reg(instr);
        let mut y_register = decode_instr_y_reg(instr);
        
        // In newer interpreters, probably because of a quirk in S-CHIP, the Vy register is ignored
        // and instead Vx is shifted in-place.
        if !self.original_shift {
            y_register = x_register;
        }

        let shifted_bit = self.v_registers[y_register] & 1;
        self.v_registers[x_register] = self.v_registers[y_register] >> 1;
        // After a shift-right, VF holds the LSB that was shifted
        self.v_registers[0xF] = shifted_bit;
    }

    /// Execute `SUBN Vx, Vy` instruction
    fn instr_8xy7(&mut self, instr: u16) {
        let x_register = decode_instr_x_reg(instr);
        let y_register = decode_instr_y_reg(instr);
        let (diff, borrow) = self.v_registers[y_register].overflowing_sub(self.v_registers[x_register]);
        
        // After performing register subtraction, VF acts as a NOT borrow flag
        self.v_registers[x_register] = diff;
        self.v_registers[0xF] = (!borrow) as u8;
    }

    /// Execute `SHL Vx, Vy` instruction
    fn instr_8xye(&mut self, instr: u16) {
        let x_register = decode_instr_x_reg(instr);
        let mut y_register = decode_instr_y_reg(instr);
        
        // In newer interpreters, probably because of a quirk in S-CHIP, the Vy register is ignored
        // and instead Vx is shifted in-place.
        if !self.original_shift {
            y_register = x_register;
        }

        let shifted_bit = (self.v_registers[y_register] >> 7) & 1;
        self.v_registers[x_register] = self.v_registers[y_register] << 1;
        // After a left-right, VF holds the MSB that was shifted
        self.v_registers[0xF] = shifted_bit;
    }

    /// Execute `SNE Vx, Vy` instruction
    fn instr_9xy0(&mut self, instr: u16) {
        let x_register = decode_instr_x_reg(instr);
        let y_register = decode_instr_y_reg(instr);

        // Skip the next instruction if the values of the registers are not equal
        if self.v_registers[x_register] != self.v_registers[y_register] {
            self.pc_register += 2;
        }
    }

    /// Execute `LD I, addr` instruction
    fn instr_annn(&mut self, instr: u16) {
        self.i_register = decode_instr_addr(instr);
    }

    /// Execute `JP V0, addr` instruction
    fn instr_bnnn(&mut self, instr: u16) {
        let jump_target = decode_instr_addr(instr) + (self.v_registers[0] as usize);
        assert!(jump_target >= MEM_RESERVED, "Jumping to the reserved memory area is not allowed");
        // assert!(jump_target%2 == 0, "Unaligned jumps are not allowed");        

        // Update pc to jump target. We subtract 2 so after the PC is incremented the jump
        // target is the next instruction that will be executed. We aren't able to jump to address
        // 0, so no wrapping is possible.
        self.pc_register = jump_target - 2;
    }

    /// Execute `RND Vx, byte` instruction
    fn instr_cxkk(&mut self, instr: u16) {
        let register_idx = decode_instr_x_reg(instr);
        let byte_imm = decode_instr_byte_imm(instr);
        self.v_registers[register_idx] = rand::thread_rng().gen::<u8>() & byte_imm;
    }

    /// Execute `DRW Vx, Vy, nibble` instruction
    fn instr_dxyn(&mut self, instr: u16) {
        let sprite_x = (self.v_registers[decode_instr_x_reg(instr)] as usize) % SCREEN_WIDTH;
        let sprite_y = (self.v_registers[decode_instr_y_reg(instr)] as usize) % SCREEN_HEIGHT;
        let sprite_height = decode_instr_nibble_imm(instr) as usize;

        // Sprite positioning details are generally inconsistent across sources online. Some claim
        // that if a sprite's (x, y) are off-screen they are wrapped, and some say they dont, some
        // claim that if if a sprite starts on-screen but then extends beyond the edge it is clipped
        // but some claim that it is wrapped to the other side. Because this seems inconsistent,
        // exposing mode toggles to the user is what we should probably do because different game
        // ROMs might assume different modes of operation. TODO: Handle this

        // We currently assume that the (x, y) of a sprite is wrapped, but that a sprite that
        // extends beyond the edge of the screen is clipped.

        assert!(self.i_register + sprite_height - 1 < MEM_SIZE,
            "Requested sprite height is out of bounds");

        let mut collision = false;
        for pixel_y in sprite_y..std::cmp::min(sprite_y+sprite_height, SCREEN_HEIGHT){
            for pixel_x in sprite_x..std::cmp::min(sprite_x+8, SCREEN_WIDTH) {
                // A sprite is a bit-packed representation of a bitmap, as such its width is 8,
                // and the number of bytes is its height.
                let sprite_row = self.memory[self.i_register + (pixel_y - sprite_y)];
                // The MSB is the leftmost pixel
                let row_bit_shift = 7 - (pixel_x - sprite_x);
                let pixel_on = ((sprite_row >> row_bit_shift) & 1) != 0;
                
                // Remember if there was any collision during the drawing
                if self.screen_buffer[pixel_y * SCREEN_WIDTH + pixel_x] && pixel_on {
                    collision = true;
                }

                // Set pixel. If a pixel is already set, we need to turn it off
                self.screen_buffer[pixel_y * SCREEN_WIDTH + pixel_x] ^= pixel_on;
            }
        }

        // When drawing sprites, VF acts as collision flag
        self.v_registers[0xF] = collision as u8;

        self.screen_dirty = true;
    }

    /// Execute `SKP Vx` instruction
    fn instr_ex9e(&mut self, instr: u16) {
        let key_digit = self.v_registers[decode_instr_x_reg(instr)];
        // Skip the next instruction if key value of reg Vx is pressed
        if self.key_state[key_digit as usize] {
            self.pc_register += 2;
        }
    }

    /// Execute `SKNP Vx` instruction
    fn instr_exa1(&mut self, instr: u16) {
        let key_digit = self.v_registers[decode_instr_x_reg(instr)];
        // Skip the next instruction if key value of reg Vx is not pressed
        if !self.key_state[key_digit as usize] {
            self.pc_register += 2;
        }
    }

    /// Execute `LD Vx, DT` instruction
    fn instr_fx07(&mut self, instr: u16) {
        self.v_registers[decode_instr_x_reg(instr)] = self.dt_register;
    }

    /// Execute `LD Vx, K` instruction
    fn instr_fx0a(&mut self, instr: u16) {
        if self.waiting_for_keypress {
            // We are already waiting for a key press, because this instruction was executed again,
            // we know we just received the capture key.
            self.v_registers[decode_instr_x_reg(instr)] = self.captured_key;
            self.waiting_for_keypress = false;
        } else{
            self.waiting_for_keypress = true;
            // We are gonna block until we capture a key, after which we want this instruction to
            // execute again. We decremnt PC so the automatic PC increment after instruction
            // execution will result in this instruction being executed again after a key press.
            self.pc_register -= 2;
        }
    }

    /// Execute `LD DT, Vx` instruction
    fn instr_fx15(&mut self, instr: u16) {
        self.dt_register = self.v_registers[decode_instr_x_reg(instr)];
    }

    /// Execute `LD ST, Vx` instruction
    fn instr_fx18(&mut self, instr: u16) {
        self.st_register = self.v_registers[decode_instr_x_reg(instr)];
    }

    /// Execute `ADD I, Vx` instruction
    fn instr_fx1e(&mut self, instr: u16) {
        self.i_register += self.v_registers[decode_instr_x_reg(instr)] as usize;
        assert!(self.i_register < MEM_SIZE, "I register overflow, do we need to handle wrapping?");
    }

    /// Execute `LD F, Vx` instruction
    fn instr_fx29(&mut self, instr: u16) {
        let hex_digit = self.v_registers[decode_instr_x_reg(instr)];
        
        // We store the font sprites at address 0, and each sprite takes up 5 bytes.
        self.i_register = 5 * (hex_digit as usize);
    }

    /// Execute `LD B, Vx` instruction
    fn instr_fx33(&mut self, instr: u16) {
        assert!(self.i_register + 2 < MEM_SIZE, "Requested BCD encoding is out of bounds");
        let reg_val = self.v_registers[decode_instr_x_reg(instr)];
        self.memory[self.i_register] = reg_val/100;
        self.memory[self.i_register+1] = (reg_val/10)%10;
        self.memory[self.i_register+2] = reg_val%10;
    }

    /// Execute `LD [I], Vx` instruction
    fn instr_fx55(&mut self, instr: u16) {
        let last_reg = decode_instr_x_reg(instr);
        
        // We need to check that I + Vx + 1 is still in bounds because we set I to the address after
        // the last stored register, and we assume I holds an in-bound address.
        assert!(self.i_register + last_reg + 1 < MEM_SIZE,
            "Requested memory store is out of bounds");

        // Store registers V0 through Vx in memory, starting at address I
        for reg in 0..last_reg+1 {
            self.memory[self.i_register + reg] = self.v_registers[reg];
        }

        // In the original CHIP-8 interpreter, the I register was incremented in the store loop.
        // Some newer interpreters don't change the I register.
        if self.original_mem_acc {
            // Update I register to hold the address after the last stored register
            self.i_register += last_reg + 1;
        }
    }

    /// Execute `LD Vx, [I]` instruction
    fn instr_fx65(&mut self, instr: u16) {
        let last_reg = decode_instr_x_reg(instr);
        
        // We need to check that I + Vx + 1 is still in bounds because we set I to the address after
        // the last loaded register, and we assume I holds an in-bound address.
        assert!(self.i_register + last_reg + 1 < MEM_SIZE,
            "Requested memory load is out of bounds");

        // Load registers V0 through Vx from memory, starting at address I
        for reg in 0..last_reg+1 {
            self.v_registers[reg] = self.memory[self.i_register + reg];
        }

        // In the original CHIP-8 interpreter, the I register was incremented in the load loop.
        // Some newer interpreters don't change the I register.
        if self.original_mem_acc {
            // Update I register to hold the address after the last stored register
            self.i_register += last_reg + 1;
        }
    }
}

/// Decodes a memory address from a CHIP-8 instruction
fn decode_instr_addr(instr: u16) -> usize {
    (instr & 0x0FFF) as usize
}

/// Decodes the first register from a CHIP-8 instruction
fn decode_instr_x_reg(instr: u16) -> usize {
    ((instr & 0x0F00) >> 8) as usize
}

/// Decodes the second register from a CHIP-8 instruction
fn decode_instr_y_reg(instr: u16) -> usize {
    ((instr & 0x00F0) >> 4) as usize
}

/// Decodes a byte-sized immediate from a CHIP-8 instruction
fn decode_instr_byte_imm(instr: u16) -> u8 {
    (instr & 0x00FF) as u8
}

/// Decodes a nibble-sized immediate from a CHIP-8 instruction
fn decode_instr_nibble_imm(instr: u16) -> u8 {
    (instr & 0x000F) as u8
}